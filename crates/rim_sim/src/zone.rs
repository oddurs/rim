//! Zones: cells the player paints. A stockpile says which items it takes,
//! and hauling fills it; a growing zone names a plant, and sowing fills it
//! (it takes no items, so no store logic sees it). They are the player's,
//! so they change only by commands.

use crate::defs::{DefDb, DefId};
use crate::filter::{Filter, FilterEdit};
use crate::map::Map;
use crate::IVec;
use serde::{Deserialize, Serialize};

/// A store a command names: a stockpile zone, or a container (DESIGN.md
/// §4f).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreRef {
    Zone(u32),
    Thing(hecs::Entity),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    /// Stable for the zone's life, never reused.
    pub id: u32,
    pub name: String,
    /// What it takes. Flattened, so a save from before filters (a zone with
    /// only `allows`) loads as one that takes any material and condition.
    #[serde(flatten)]
    pub filter: Filter,
    /// Its place on the store priority scale: stacks move only to a store
    /// at a higher level (DESIGN.md §4f). A zone saved before levels is at
    /// core's Normal.
    #[serde(default = "normal", skip_serializing_if = "is_normal")]
    pub level: u8,
    /// A growing zone: the plant sown on its empty cells, and harvested
    /// when grown. Its filter takes nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plant: Option<DefId>,
}

fn normal() -> u8 {
    1
}

fn is_normal(level: &u8) -> bool {
    *level == normal()
}

impl Zone {
    /// Whether it takes this thing in some material and condition.
    pub fn takes(&self, def: DefId) -> bool {
        self.filter.takes_thing(def)
    }

    /// Whether it takes a stack of `def` made of `made_of` at `hp`.
    pub fn keeps(&self, defs: &DefDb, def: DefId, made_of: Option<DefId>, hp: Option<i32>) -> bool {
        self.filter.takes(defs, def, made_of, hp)
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

/// The map indices from `a` to `b` on `a`'s level, clamped to the map,
/// row by row. Nothing on a level the map doesn't have.
fn rect(map: &Map, a: IVec, b: IVec) -> impl Iterator<Item = usize> + '_ {
    let (x0, x1) = (a.x.min(b.x).max(0), a.x.max(b.x).min(map.w - 1));
    let (y0, y1) = (a.y.min(b.y).max(0), a.y.max(b.y).min(map.h - 1));
    let y1 = if map.levels().contains(&a.z) { y1 } else { y0 - 1 };
    (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| map.idx(IVec::at(x, y, a.z))))
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
        let level = defs.store_priority.default;
        self.list.push(Zone {
            id,
            name: format!("Stockpile {id}"),
            filter: Filter::everything(defs),
            level,
            plant: None,
        });
        self.paint(map, a, b, Some(id));
        id
    }

    /// The cells `paint` would change, as map indices: with a zone, the
    /// cells from `a` to `b` in no zone; with none, the cells in any. A
    /// preview reads this, so it shows what painting does.
    pub fn painted(&self, map: &Map, a: IVec, b: IVec, id: Option<u32>) -> Vec<usize> {
        rect(map, a, b).filter(|&i| (self.cells[i] == 0) == id.is_some()).collect()
    }

    /// A new growing zone of `plant` over the free cells from `a` to `b`.
    pub fn create_growing(&mut self, defs: &DefDb, map: &Map, a: IVec, b: IVec, plant: DefId) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let name = format!("{} field {id}", upper_first(&defs.thing(plant).label));
        let level = defs.store_priority.default;
        self.list.push(Zone { id, name, filter: Filter::nothing(), level, plant: Some(plant) });
        self.paint(map, a, b, Some(id));
        id
    }

    /// Change what a growing zone sows. A stockpile stays a stockpile.
    pub fn set_plant(&mut self, id: u32, plant: DefId) {
        if let Some(z) = self.list.iter_mut().find(|z| z.id == id && z.plant.is_some()) {
            z.plant = Some(plant);
        }
    }

    /// Growing zones and their cells, oldest zone first, cells in map order.
    pub fn growing(&self) -> impl Iterator<Item = (DefId, u32)> + '_ {
        self.members().filter_map(|(z, c)| z.plant.map(|p| (p, c)))
    }

    /// Put the cells from `a` to `b` in zone `id`, or in none. Painting
    /// never takes a cell from another zone: clear it first. A zone left
    /// with no cells is gone.
    pub fn paint(&mut self, map: &Map, a: IVec, b: IVec, id: Option<u32>) {
        if id.is_some_and(|id| self.get(id).is_none()) {
            return;
        }
        for i in self.painted(map, a, b, id) {
            self.cells[i] = id.unwrap_or(0);
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

    /// Change what a zone takes.
    pub fn edit(&mut self, defs: &DefDb, id: u32, edit: FilterEdit) {
        if let Some(z) = self.list.iter_mut().find(|z| z.id == id) {
            z.filter.edit(defs, edit);
        }
    }

    /// Put a zone at a level, within the scale.
    pub fn set_level(&mut self, defs: &DefDb, id: u32, level: u8) {
        let top = defs.store_priority.labels.len().saturating_sub(1) as u8;
        if let Some(z) = self.list.iter_mut().find(|z| z.id == id) {
            z.level = level.min(top);
        }
    }

    /// The single zone the cells from `a` to `b` touch, if exactly one.
    pub fn touched(&self, map: &Map, a: IVec, b: IVec) -> Option<u32> {
        let mut found = None;
        for i in rect(map, a, b) {
            match (self.cells[i], found) {
                (0, _) => {}
                (id, None) => found = Some(id),
                (id, Some(f)) if id != f => return None,
                _ => {}
            }
        }
        found
    }

    /// Make a loaded set of zones consistent, whatever a hand-edited save
    /// says: filters sorted without repeats, cells only in zones that exist,
    /// no zone without cells, and ids that won't be handed out again.
    pub fn tidy(&mut self) {
        for z in &mut self.list {
            z.filter.tidy();
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
            let plant = z.plant.map_or(0, |p| p as u64 + 1) << 48;
            h = z.filter.hash(mix(h ^ z.id as u64 ^ (z.level as u64) << 40 ^ plant));
        }
        for (i, &c) in self.cells.iter().enumerate().filter(|(_, &c)| c != 0) {
            h = mix(h ^ (i as u64) << 20 ^ c as u64);
        }
        h
    }
}

fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
}
