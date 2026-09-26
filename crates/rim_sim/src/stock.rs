//! What the colony has on the map, kept as stacks change rather than
//! counted when asked (DESIGN.md §4f, Cost). Two indexes:
//!
//! - the **ledger**: units of each item def by material, and how many of
//!   them lie where a stockpile keeps them;
//! - **holdings**: how many stacks of each item def lie in each 32×32
//!   chunk, so the nearest flint is found by looking in the chunks that
//!   hold flint, nearest first.
//!
//! Every stack change on the item layer goes through `World`'s few choke
//! points, which call `change`. Derived, never saved: rebuilt on load, and
//! a debug build checks it against a full count every 1,000 ticks.

use crate::defs::DefId;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stock {
    /// By item def: units on the map, by material (`None` first).
    units: Vec<BTreeMap<Option<DefId>, u32>>,
    /// By item def: units lying where a stockpile keeps them.
    stored: Vec<u32>,
    /// By item def: stacks in each chunk, by chunk index.
    holdings: Vec<BTreeMap<u32, u32>>,
}

/// One stack's worth of change: `units` more or fewer of it, and one more
/// or fewer stack when a stack appears or goes.
#[derive(Clone, Copy, Debug)]
pub struct Change {
    pub def: DefId,
    pub made_of: Option<DefId>,
    pub chunk: u32,
    /// Whether a stockpile at its cell keeps it.
    pub kept: bool,
    pub units: i64,
    pub stacks: i64,
}

impl Stock {
    pub fn new(things: usize) -> Stock {
        Stock { units: vec![BTreeMap::new(); things], stored: vec![0; things], holdings: vec![BTreeMap::new(); things] }
    }

    pub fn change(&mut self, c: Change) {
        let d = c.def as usize;
        let add = |n: &mut u32, by: i64| *n = (*n as i64 + by).max(0) as u32;
        let u = self.units[d].entry(c.made_of).or_default();
        add(u, c.units);
        if *u == 0 {
            self.units[d].remove(&c.made_of);
        }
        if c.kept {
            add(&mut self.stored[d], c.units);
        }
        if c.stacks != 0 {
            let h = self.holdings[d].entry(c.chunk).or_default();
            add(h, c.stacks);
            if *h == 0 {
                self.holdings[d].remove(&c.chunk);
            }
        }
    }

    /// Units of `def` on the map, any material.
    pub fn on_map(&self, def: DefId) -> u32 {
        self.units.get(def as usize).map_or(0, |m| m.values().sum())
    }

    /// Units of `def` made of `made_of` on the map.
    pub fn on_map_of(&self, def: DefId, made_of: Option<DefId>) -> u32 {
        self.units.get(def as usize).and_then(|m| m.get(&made_of)).copied().unwrap_or(0)
    }

    /// Each material `def` is on the map in, with its units, `None` first.
    pub fn materials(&self, def: DefId) -> impl Iterator<Item = (Option<DefId>, u32)> + '_ {
        self.units.get(def as usize).into_iter().flat_map(|m| m.iter().map(|(&k, &v)| (k, v)))
    }

    /// Units of `def` lying where a stockpile keeps them.
    pub fn stored(&self, def: DefId) -> u32 {
        self.stored.get(def as usize).copied().unwrap_or(0)
    }

    /// Forget what's stored, to count it again after the zones changed.
    pub fn clear_stored(&mut self) {
        self.stored.iter_mut().for_each(|n| *n = 0);
    }

    pub fn add_stored(&mut self, def: DefId, units: u32) {
        self.stored[def as usize] += units;
    }

    /// The chunks holding stacks of `def`, by index, with how many.
    pub fn chunks(&self, def: DefId) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.holdings.get(def as usize).into_iter().flat_map(|m| m.iter().map(|(&k, &v)| (k, v)))
    }
}
