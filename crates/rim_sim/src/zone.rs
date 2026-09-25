//! Stockpile zones: cells the player paints, and which items each takes.
//! They are the player's, so they change only by commands; hauling fills
//! them.

use crate::defs::{Category, DefDb, DefId};
use crate::map::Map;
use crate::IVec;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    /// Stable for the zone's life, never reused.
    pub id: u32,
    pub name: String,
    /// The item defs it takes, sorted.
    pub allows: Vec<DefId>,
}

impl Zone {
    pub fn takes(&self, def: DefId) -> bool {
        self.allows.binary_search(&def).is_ok()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Zones {
    /// In the order they were made.
    pub list: Vec<Zone>,
    pub next_id: u32,
    /// The zone each cell belongs to; 0 for none.
    pub cells: Vec<u32>,
    /// Each zone's cells, by map index, rebuilt whenever cells change so a
    /// search walks a zone rather than the map. Derived, never saved.
    #[serde(skip)]
    members: Vec<(u32, Vec<u32>)>,
}

impl Zones {
    pub fn new(cells: usize) -> Zones {
        Zones { list: Vec::new(), next_id: 1, cells: vec![0; cells], members: Vec::new() }
    }

    pub fn get(&self, id: u32) -> Option<&Zone> {
        self.list.iter().find(|z| z.id == id)
    }

    /// The zone a cell is in.
    pub fn at(&self, map: &Map, p: IVec) -> Option<&Zone> {
        if !map.inb(p) {
            return None;
        }
        match self.cells[map.idx(p)] {
            0 => None,
            id => self.get(id),
        }
    }

    /// A new zone over the free cells from `a` to `b`, taking every item
    /// there is now; an item a mod adds later is the player's to allow.
    /// Returns its id.
    pub fn create(&mut self, defs: &DefDb, map: &Map, a: IVec, b: IVec) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let allows = (0..defs.things.len() as DefId).filter(|&d| defs.thing(d).category == Category::Item).collect();
        self.list.push(Zone { id, name: format!("Stockpile {id}"), allows });
        self.paint(map, a, b, Some(id));
        id
    }

    /// Put the cells from `a` to `b` in zone `id`, or in none. Painting
    /// never takes a cell from another zone: clear it first. A zone left
    /// with no cells is gone.
    pub fn paint(&mut self, map: &Map, a: IVec, b: IVec, id: Option<u32>) {
        if id.is_some_and(|id| self.get(id).is_none()) {
            return;
        }
        let (x0, x1) = (a.x.min(b.x).max(0), a.x.max(b.x).min(map.w - 1));
        let (y0, y1) = (a.y.min(b.y).max(0), a.y.max(b.y).min(map.h - 1));
        for y in y0..=y1 {
            for x in x0..=x1 {
                let i = map.idx(IVec::new(x, y));
                match id {
                    Some(id) if self.cells[i] == 0 => self.cells[i] = id,
                    Some(_) => {}
                    None => self.cells[i] = 0,
                }
            }
        }
        let cells = &self.cells;
        self.list.retain(|z| cells.contains(&z.id));
        self.index();
    }

    /// Every zone's cells, oldest zone first, cells in map order.
    pub fn members(&self) -> impl Iterator<Item = (&Zone, u32)> + '_ {
        self.members.iter().flat_map(move |(id, cells)| {
            let z = self.get(*id).expect("members follow the list");
            cells.iter().map(move |&c| (z, c))
        })
    }

    fn index(&mut self) {
        let mut members: Vec<(u32, Vec<u32>)> = self.list.iter().map(|z| (z.id, Vec::new())).collect();
        for (i, &c) in self.cells.iter().enumerate().filter(|(_, &c)| c != 0) {
            if let Some(m) = members.iter_mut().find(|m| m.0 == c) {
                m.1.push(i as u32);
            }
        }
        self.members = members;
    }

    /// Let a zone take an item, or stop it.
    pub fn allow(&mut self, id: u32, def: DefId, on: bool) {
        let Some(z) = self.list.iter_mut().find(|z| z.id == id) else { return };
        match (z.allows.binary_search(&def), on) {
            (Err(i), true) => z.allows.insert(i, def),
            (Ok(i), false) => {
                z.allows.remove(i);
            }
            _ => {}
        }
    }

    /// The single zone the cells from `a` to `b` touch, if exactly one.
    pub fn touched(&self, map: &Map, a: IVec, b: IVec) -> Option<u32> {
        let (x0, x1) = (a.x.min(b.x).max(0), a.x.max(b.x).min(map.w - 1));
        let (y0, y1) = (a.y.min(b.y).max(0), a.y.max(b.y).min(map.h - 1));
        let mut found = None;
        for y in y0..=y1 {
            for x in x0..=x1 {
                match (self.cells[map.idx(IVec::new(x, y))], found) {
                    (0, _) => {}
                    (id, None) => found = Some(id),
                    (id, Some(f)) if id != f => return None,
                    _ => {}
                }
            }
        }
        found
    }

    /// Make a loaded set of zones consistent, whatever a hand-edited save
    /// says: filters sorted without repeats, cells only in zones that exist,
    /// no zone without cells, and ids that won't be handed out again.
    pub fn tidy(&mut self) {
        for z in &mut self.list {
            z.allows.sort_unstable();
            z.allows.dedup();
        }
        let mut seen = Vec::new();
        self.list.retain(|z| {
            let fresh = !seen.contains(&z.id);
            seen.push(z.id);
            fresh && z.id != 0
        });
        let ids: Vec<u32> = self.list.iter().map(|z| z.id).collect();
        for c in &mut self.cells {
            if *c != 0 && !ids.contains(c) {
                *c = 0;
            }
        }
        let cells = &self.cells;
        self.list.retain(|z| cells.contains(&z.id));
        self.next_id = self.next_id.max(ids.iter().max().map_or(1, |m| m + 1));
        self.index();
    }

    pub fn hash(&self, mut h: u64) -> u64 {
        use crate::rng::mix;
        h = mix(h ^ self.next_id as u64);
        for z in &self.list {
            h = mix(h ^ z.id as u64);
            for &d in &z.allows {
                h = mix(h ^ d as u64);
            }
        }
        for (i, &c) in self.cells.iter().enumerate().filter(|(_, &c)| c != 0) {
            h = mix(h ^ (i as u64) << 20 ^ c as u64);
        }
        h
    }
}
