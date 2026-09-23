//! Field layers: scalar values over the grid (temperature, light, beauty...).
//!
//! A field's value at a cell is its *base* plus *stamped* emitter
//! contributions:
//!
//! - **Base** is the outdoor ambient (set by scripts) for cells under open
//!   sky. Inside an enclosed room it depends on the field's indoor mode: the
//!   room's own simulated value (`room`), nothing (`none`), or the same as
//!   outdoors (`outdoor`).
//! - **Stamped** comes from emitters: things whose def has `emit = [...]`.
//!   For `room` fields the stamp only applies outside enclosed rooms: inside,
//!   the emitter has already pushed the room's own value.
//!   An emitter adds to cells within its radius, fading with walking
//!   distance, so walls block it. Each emitter remembers exactly which cells
//!   it touched, so adding or removing one costs only its own footprint, and a
//!   wall change re-stamps only the emitters within reach of it.
//!
//! Room values cost O(rooms) per update, not O(cells). Values are stored in
//! hundredths as integers so the simulation stays deterministic.

use crate::defs::{DefDb, DefId, IndoorMode};
use crate::map::{Map, NEIGHBORS8};
use crate::{IVec, TICKS_PER_DAY};
use hecs::Entity;
use std::collections::VecDeque;

/// Field values are stored in hundredths.
pub const FIXED: f64 = 100.0;

/// How often room-state fields update, in ticks.
pub const ROOM_INTERVAL: u64 = 60;

struct Emitter {
    entity: Entity,
    field: usize,
    pos: IVec,
    /// Full strength at the source, in hundredths.
    amount: i32,
    radius: u32,
    /// Room fields: the room value this emitter stops pushing past, in hundredths.
    cap: Option<i32>,
    /// The contribution this emitter made to each cell, so it can be undone exactly.
    cells: Vec<(u32, i32)>,
}

pub struct Layer {
    /// Emitter contributions per cell.
    pub stamped: Vec<i32>,
    /// Open-sky value.
    pub ambient: i32,
    /// Per-room value for `indoor = "room"` fields, indexed by room id - 1.
    pub rooms: Vec<i32>,
}

pub struct Fields {
    pub layers: Vec<Layer>,
    emitters: Vec<Emitter>,
    seen_rebuilds: u64,
    /// Scratch for the stamping flood fill: generation-stamped distances.
    dist: Vec<u32>,
    dist_gen: Vec<u32>,
    gen: u32,
    queue: VecDeque<u32>,
    /// Cells re-stamped by the last `update`, for tests and the profiler.
    pub restamped: u64,
}

impl Fields {
    pub fn new(defs: &DefDb, cells: usize) -> Self {
        Fields {
            layers: defs
                .fields
                .iter()
                .map(|f| Layer { stamped: vec![0; cells], ambient: (f.ambient * FIXED) as i32, rooms: Vec::new() })
                .collect(),
            emitters: Vec::new(),
            seen_rebuilds: u64::MAX,
            dist: vec![0; cells],
            dist_gen: vec![0; cells],
            gen: 0,
            queue: VecDeque::new(),
            restamped: 0,
        }
    }

    /// Register the emitters of a thing that now exists at `pos`.
    pub fn add_emitters(&mut self, defs: &DefDb, map: &Map, entity: Entity, thing: DefId, pos: IVec) {
        for em in &defs.thing(thing).emit {
            let mut e = Emitter {
                entity,
                field: em.field_r as usize,
                pos,
                amount: (em.amount * FIXED) as i32,
                radius: em.radius,
                cap: em.cap.map(|c| (c * FIXED) as i32),
                cells: Vec::new(),
            };
            self.stamp(map, &mut e);
            self.emitters.push(e);
        }
    }

    pub fn remove_emitters(&mut self, entity: Entity) {
        let mut i = 0;
        while i < self.emitters.len() {
            if self.emitters[i].entity == entity {
                let e = self.emitters.remove(i);
                let layer = &mut self.layers[e.field].stamped;
                for (c, v) in e.cells {
                    layer[c as usize] -= v;
                }
            } else {
                i += 1;
            }
        }
    }

    /// Flood out from the emitter, adding a falloff by walking distance.
    fn stamp(&mut self, map: &Map, e: &mut Emitter) {
        self.gen = self.gen.wrapping_add(1);
        if self.gen == 0 {
            self.dist_gen.iter_mut().for_each(|g| *g = 0);
            self.gen = 1;
        }
        let gen = self.gen;
        let layer = &mut self.layers[e.field].stamped;
        let start = map.idx(e.pos) as u32;
        self.dist[start as usize] = 0;
        self.dist_gen[start as usize] = gen;
        self.queue.clear();
        self.queue.push_back(start);
        let span = e.radius as i64 + 1;
        while let Some(c) = self.queue.pop_front() {
            let d = self.dist[c as usize];
            let v = (e.amount as i64 * (span - d as i64) / span) as i32;
            layer[c as usize] += v;
            e.cells.push((c, v));
            if d >= e.radius {
                continue;
            }
            let p = map.pos(c as usize);
            for (dx, dy) in NEIGHBORS8 {
                let q = p.offset(dx, dy);
                if !map.inb(q) {
                    continue;
                }
                let qi = map.idx(q);
                // Walls and doors stop it; so do corners it can't slip past.
                if self.dist_gen[qi] == gen || map.blocks_fields(qi) {
                    continue;
                }
                if dx != 0
                    && dy != 0
                    && (map.blocks_fields(map.idx(p.offset(dx, 0))) || map.blocks_fields(map.idx(p.offset(0, dy))))
                {
                    continue;
                }
                self.dist_gen[qi] = gen;
                self.dist[qi] = d + 1;
                self.queue.push_back(qi as u32);
            }
        }
    }

    fn restamp_near(&mut self, map: &Map, changed: &[u32]) {
        for k in 0..self.emitters.len() {
            let (pos, reach) = (self.emitters[k].pos, self.emitters[k].radius as i32 + 1);
            if !changed.iter().any(|&c| map.pos(c as usize).chebyshev(pos) <= reach) {
                continue;
            }
            let mut e = std::mem::replace(
                &mut self.emitters[k],
                Emitter { entity: Entity::DANGLING, field: 0, pos, amount: 0, radius: 0, cap: None, cells: Vec::new() },
            );
            let layer = &mut self.layers[e.field].stamped;
            for (c, v) in e.cells.drain(..) {
                layer[c as usize] -= v;
            }
            self.stamp(map, &mut e);
            self.restamped += e.cells.len() as u64;
            self.emitters[k] = e;
        }
    }

    /// Call once per tick after rooms are current.
    pub fn update(&mut self, defs: &DefDb, map: &mut Map, tick: u64) {
        let changed = map.take_changed_cells();
        if !changed.is_empty() {
            self.restamp_near(map, &changed);
        }
        if map.room_rebuilds != self.seen_rebuilds {
            self.carry_rooms_over(defs, map);
            self.seen_rebuilds = map.room_rebuilds;
        }
        if tick.is_multiple_of(ROOM_INTERVAL) {
            self.step_rooms(defs, map);
        }
    }

    /// Rooms were rebuilt: each new room starts at the cell-weighted average
    /// of whatever its cells held before, so a wall elsewhere changes nothing.
    fn carry_rooms_over(&mut self, defs: &DefDb, map: &Map) {
        let n = map.room_count();
        for (fi, fd) in defs.fields.iter().enumerate() {
            if fd.indoor != IndoorMode::Room {
                continue;
            }
            let layer = &mut self.layers[fi];
            let old = std::mem::take(&mut layer.rooms);
            let mut sum = vec![0i64; n];
            let mut count = vec![0i64; n];
            for i in 0..map.w as usize * map.h as usize {
                let (new_id, old_id) = map.room_ids(i);
                if new_id == 0 {
                    continue;
                }
                let before = old.get(old_id.wrapping_sub(1) as usize).copied().unwrap_or(layer.ambient);
                sum[new_id as usize - 1] += before as i64;
                count[new_id as usize - 1] += 1;
            }
            layer.rooms =
                (0..n).map(|r| if count[r] > 0 { (sum[r] / count[r]) as i32 } else { layer.ambient }).collect();
        }
    }

    /// Room values leak toward outdoors and are pushed by emitters inside.
    fn step_rooms(&mut self, defs: &DefDb, map: &Map) {
        let hours = ROOM_INTERVAL as f64 * 24.0 / TICKS_PER_DAY as f64;
        for (fi, fd) in defs.fields.iter().enumerate() {
            if fd.indoor != IndoorMode::Room {
                continue;
            }
            let n = map.room_count();
            let layer = &mut self.layers[fi];
            layer.rooms.resize(n, layer.ambient);
            let mut heat = vec![0f64; n];
            for e in self.emitters.iter().filter(|e| e.field == fi) {
                let Some(r) = map.room_at(e.pos) else { continue };
                let now = layer.rooms[r.id as usize - 1];
                // A capped emitter stops pushing once the room reaches its cap.
                let spent = e.cap.is_some_and(|c| if e.amount >= 0 { now >= c } else { now <= c });
                if !spent {
                    heat[r.id as usize - 1] += e.amount as f64 * fd.room_gain;
                }
            }
            let ambient = layer.ambient;
            for (r, (value, heat)) in layer.rooms.iter_mut().zip(&heat).enumerate() {
                let room = map.room_by_id(r as u32 + 1);
                if !room.enclosed() {
                    *value = ambient;
                    continue;
                }
                let v = *value as f64;
                let change = (ambient as f64 - v) * fd.leak_per_hour + heat / room.cells as f64;
                *value = (v + change * hours).round() as i32;
            }
        }
    }

    /// Value at a cell, in hundredths.
    pub fn value_fixed(&self, defs: &DefDb, map: &Map, field: usize, p: IVec) -> i32 {
        if !map.inb(p) {
            return 0;
        }
        let layer = &self.layers[field];
        let stamped = layer.stamped[map.idx(p)];
        let indoors = map.room_at(p).filter(|r| r.enclosed());
        match (defs.fields[field].indoor, indoors) {
            (_, None) | (IndoorMode::Outdoor, _) => layer.ambient + stamped,
            (IndoorMode::None, Some(_)) => stamped,
            // The room's value already includes what its emitters put in;
            // adding their local stamp too would count the same fire twice.
            (IndoorMode::Room, Some(r)) => layer.rooms.get(r.id as usize - 1).copied().unwrap_or(layer.ambient),
        }
    }

    /// Value at a cell, in the field's own units.
    pub fn value(&self, defs: &DefDb, map: &Map, field: usize, p: IVec) -> f64 {
        self.value_fixed(defs, map, field, p) as f64 / FIXED
    }

    pub fn set_ambient(&mut self, field: usize, v: f64) {
        self.layers[field].ambient = (v * FIXED).round() as i32;
    }

    pub fn ambient(&self, field: usize) -> f64 {
        self.layers[field].ambient as f64 / FIXED
    }

    pub fn emitter_count(&self) -> usize {
        self.emitters.len()
    }
}
