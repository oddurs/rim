//! Where things go (DESIGN.md §4f, Sorting only climbs). Stores are
//! stockpile zones and containers; each has a level, and a stack moves only to a
//! store at a strictly higher level that takes it, nearest within that
//! level. So two stores at one level never trade, and nothing thrashes.
//!
//! The index here answers the haul search's questions without walking the
//! map or every zone cell:
//!
//! - **accepts**, by item def: the stores that take it, highest level first;
//! - **open**, by zone: its cells with nothing on them;
//! - **partial**, by zone and stack kind: its cells with a stack below its
//!   limit, so hauls merge before they spread;
//! - **unsorted**, by chunk: stacks on the item layer that some zone takes
//!   at a higher level than the one they lie in (or lie in none).
//!
//! Derived, never saved. Zones changing rebuilds it; a stack appearing or
//! going updates it; open cells are checked again when the map's
//! passability or fixtures change (its revision), and every use of a cell
//! checks `room_for`, so a stale entry costs a skip, never a wrong haul.

use crate::defs::DefId;
use crate::map::Map;
use crate::zone::Zones;
use std::collections::{BTreeMap, BTreeSet};

/// What kind of stack: thing and material. Stacks of one kind merge.
pub type Kind = (DefId, Option<DefId>);

/// A store: a stockpile zone by id, or a container by entity bits. Zones
/// sort before containers at one level, then by id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StoreKey {
    Zone(u32),
    Thing(u64),
}

/// A container as the index sees it.
#[derive(Clone, Copy, Debug)]
pub struct ContainerAt<'a> {
    pub entity: u64,
    pub level: u8,
    pub allows: &'a [DefId],
    pub chunk: u32,
}

#[derive(Clone, Debug, Default)]
pub struct StoreIndex {
    accepts: Vec<Vec<(u8, StoreKey)>>,
    /// Containers by chunk, so a search for the nearest stack looks in them.
    containers: BTreeMap<u32, BTreeSet<u64>>,
    open: BTreeMap<u32, BTreeSet<u32>>,
    partial: BTreeMap<(u32, Kind), BTreeSet<u32>>,
    unsorted: BTreeMap<u32, BTreeSet<u64>>,
    /// The map revision the open cells were last checked against.
    pub seen_revision: u64,
}

/// Two indexes are equal when they say the same things; the map revision
/// they were last checked against is bookkeeping.
impl PartialEq for StoreIndex {
    fn eq(&self, o: &Self) -> bool {
        (&self.accepts, &self.containers, &self.open, &self.partial, &self.unsorted)
            == (&o.accepts, &o.containers, &o.open, &o.partial, &o.unsorted)
    }
}

impl StoreIndex {
    /// The stores taking `def`, highest level first, then zones oldest
    /// first, then containers by id.
    pub fn accepts(&self, def: DefId) -> &[(u8, StoreKey)] {
        self.accepts.get(def as usize).map_or(&[], Vec::as_slice)
    }

    /// The highest level any zone takes `def` at.
    pub fn best_level(&self, def: DefId) -> Option<u8> {
        self.accepts(def).first().map(|&(l, _)| l)
    }

    pub fn open(&self, zone: u32) -> impl Iterator<Item = u32> + '_ {
        self.open.get(&zone).into_iter().flatten().copied()
    }

    pub fn partial(&self, zone: u32, kind: Kind) -> impl Iterator<Item = u32> + '_ {
        self.partial.get(&(zone, kind)).into_iter().flatten().copied()
    }

    /// Chunks holding stacks with somewhere better to be, and those stacks
    /// as entity bits, in id order.
    pub fn unsorted(&self) -> impl Iterator<Item = (u32, &BTreeSet<u64>)> + '_ {
        self.unsorted.iter().map(|(&c, s)| (c, s))
    }

    pub fn unsorted_count(&self) -> usize {
        self.unsorted.values().map(BTreeSet::len).sum()
    }

    /// Containers in a chunk, as entity bits.
    pub fn containers_in(&self, chunk: u32) -> impl Iterator<Item = u64> + '_ {
        self.containers.get(&chunk).into_iter().flatten().copied()
    }

    /// Everything that depends on the stores' filters and levels.
    pub fn rebuild_accepts(&mut self, zones: &Zones, containers: &[ContainerAt], things: usize) {
        let mut accepts: Vec<Vec<(u8, StoreKey)>> = vec![Vec::new(); things];
        let mut add = |d: DefId, entry| {
            if let Some(v) = accepts.get_mut(d as usize) {
                v.push(entry);
            }
        };
        for z in &zones.list {
            for &d in &z.filter.allows {
                add(d, (z.level, StoreKey::Zone(z.id)));
            }
        }
        self.containers.clear();
        for c in containers {
            for &d in c.allows {
                add(d, (c.level, StoreKey::Thing(c.entity)));
            }
            self.containers.entry(c.chunk).or_default().insert(c.entity);
        }
        for v in &mut accepts {
            v.sort_by_key(|&(l, k)| (std::cmp::Reverse(l), k));
        }
        self.accepts = accepts;
    }

    /// Open cells of every zone, from scratch.
    pub fn rebuild_open(&mut self, zones: &Zones, map: &Map) {
        let mut open: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
        for (z, c) in zones.members() {
            let p = map.pos(c as usize);
            if map.item_at(p).is_none() && map.passable(p) && map.fixture_at(p).is_none() {
                open.entry(z.id).or_default().insert(c);
            }
        }
        self.open = open;
        self.seen_revision = map.revision;
    }

    pub fn clear_stacks(&mut self) {
        self.partial.clear();
        self.unsorted.clear();
    }

    /// A stack appeared on a cell (or changed enough to recheck): it takes
    /// the cell from the open set, sits in its zone's partial set while
    /// below its limit, and is unsorted if somewhere better takes it.
    pub fn stack_here(&mut self, s: StackAt) {
        if let Some(z) = s.zone {
            if let Some(o) = self.open.get_mut(&z) {
                o.remove(&s.cell);
                if o.is_empty() {
                    self.open.remove(&z);
                }
            }
            let key = (z, s.kind);
            if s.below_limit {
                self.partial.entry(key).or_default().insert(s.cell);
            } else if let Some(set) = self.partial.get_mut(&key) {
                set.remove(&s.cell);
                if set.is_empty() {
                    self.partial.remove(&key);
                }
            }
        }
        let better = self.best_level(s.kind.0).is_some_and(|b| s.level.is_none_or(|l| b > l));
        let set = self.unsorted.entry(s.chunk).or_default();
        if better {
            set.insert(s.entity);
        } else {
            set.remove(&s.entity);
            if set.is_empty() {
                self.unsorted.remove(&s.chunk);
            }
        }
    }

    /// A stack left its cell: the cell opens again if nothing else is in
    /// the way (checked when used), and the stack is no longer anywhere.
    pub fn stack_gone(&mut self, s: StackAt) {
        if let Some(z) = s.zone {
            self.open.entry(z).or_default().insert(s.cell);
            if let Some(set) = self.partial.get_mut(&(z, s.kind)) {
                set.remove(&s.cell);
                if set.is_empty() {
                    self.partial.remove(&(z, s.kind));
                }
            }
        }
        if let Some(set) = self.unsorted.get_mut(&s.chunk) {
            set.remove(&s.entity);
            if set.is_empty() {
                self.unsorted.remove(&s.chunk);
            }
        }
    }
}

/// A stack on the item layer, as the index sees it.
#[derive(Clone, Copy, Debug)]
pub struct StackAt {
    pub entity: u64,
    pub kind: Kind,
    pub cell: u32,
    pub chunk: u32,
    /// The zone its cell is in, if any.
    pub zone: Option<u32>,
    /// The level of the zone it lies in, if that zone keeps it.
    pub level: Option<u8>,
    pub below_limit: bool,
}
