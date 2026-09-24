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
use crate::terms::{self, Env, Q};
use crate::{IVec, TICKS_PER_DAY};
use hecs::Entity;
use std::collections::VecDeque;

/// Field values are stored in hundredths.
pub const FIXED: f64 = 100.0;

/// How often room-state fields update, in ticks.
pub const ROOM_INTERVAL: u64 = 60;

/// How often outdoor values are recomputed from their terms and pushes.
pub const AMBIENT_INTERVAL: u64 = 20;

/// A named contribution to a field's outdoor value (`rim.push_ambient`).
/// It eases from `from` to `to` over `ease` ticks starting at `start`, and
/// after `until` it eases back out to zero and disappears.
#[derive(Clone, Debug, PartialEq)]
pub struct Push {
    pub key: String,
    pub from: i64,
    pub to: i64,
    pub start: u64,
    pub ease: u64,
    pub until: Option<u64>,
    /// Easing out after expiring or being cleared; removed at zero.
    pub fading: bool,
}

impl Push {
    /// Current value in `Q` units.
    pub fn value(&self, tick: u64) -> i64 {
        if self.ease == 0 || tick >= self.start + self.ease {
            return self.to;
        }
        let t = tick.saturating_sub(self.start) as i64;
        self.from + (self.to - self.from) * t / self.ease as i64
    }
}

/// Outdoor-value state for one field.
#[derive(Clone, Debug, Default)]
pub struct Atmos {
    /// Set by `rim.set_ambient`: overrides terms and pushes (tests, tools).
    pub pin: Option<i64>,
    pub pushes: Vec<Push>,
    /// Last computed outdoor value, `Q` units.
    pub value: i64,
}

/// What terms see when outdoor values are computed.
struct AmbEnv<'a> {
    year: i64,
    hour: i64,
    tick: u64,
    seed: u64,
    vals: &'a [i64],
}

impl Env for AmbEnv<'_> {
    fn year(&self) -> i64 {
        self.year
    }
    fn hour(&self) -> i64 {
        self.hour
    }
    fn ambient(&self, f: usize) -> i64 {
        self.vals[f]
    }
    fn tick(&self) -> u64 {
        self.tick
    }
    fn seed(&self) -> u64 {
        self.seed
    }
}

/// The clock, as terms see it.
#[derive(Clone, Copy, Debug)]
pub struct Clock {
    pub tick: u64,
    /// Fraction of the year, 0..Q.
    pub year: i64,
    /// Hour of day, 0..24·Q.
    pub hour: i64,
    pub seed: u64,
}

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
    /// Per-room multiplier on `leak_per_hour`, from what encloses the room.
    /// 1.0 for a boundary that says nothing.
    pub leak_mult: Vec<f64>,
    /// Per-room fraction of the outdoor value that gets in through the
    /// boundary, for fields that are otherwise dark indoors.
    pub pass: Vec<f64>,
}

pub struct Fields {
    pub layers: Vec<Layer>,
    /// Outdoor-value state per field: pins, pushes, the last value.
    pub atmos: Vec<Atmos>,
    /// When outdoor values were last computed, so a breakdown matches them.
    last_clock: Option<Clock>,
    emitters: Vec<Emitter>,
    seen_rebuilds: u64,
    /// Scratch for the stamping flood fill: generation-stamped distances.
    dist: Vec<u32>,
    dist_gen: Vec<u32>,
    gen: u32,
    queue: VecDeque<u32>,
    /// Cells re-stamped by the last `update`, for tests and the profiler.
    pub restamped: u64,
    /// Bumped whenever any emitter's stamp changes, so renderers can cache.
    pub revision: u64,
}

impl Fields {
    pub fn new(defs: &DefDb, cells: usize) -> Self {
        Fields {
            layers: defs
                .fields
                .iter()
                .map(|f| Layer {
                    stamped: vec![0; cells],
                    ambient: (f.base * FIXED) as i32,
                    rooms: Vec::new(),
                    leak_mult: Vec::new(),
                    pass: Vec::new(),
                })
                .collect(),
            atmos: defs.fields.iter().map(|f| Atmos { value: terms::to_q(f.base), ..Default::default() }).collect(),
            last_clock: None,
            emitters: Vec::new(),
            seen_rebuilds: u64::MAX,
            dist: vec![0; cells],
            dist_gen: vec![0; cells],
            gen: 0,
            queue: VecDeque::new(),
            restamped: 0,
            revision: 0,
        }
    }

    /// Rooms were rebuilt: every room starts from a boundary that says
    /// nothing until `set_boundary` fills it in.
    pub fn reset_boundaries(&mut self, rooms: usize) {
        for layer in &mut self.layers {
            layer.leak_mult = vec![1.0; rooms];
            layer.pass = vec![0.0; rooms];
        }
    }

    pub fn set_boundary(&mut self, field: usize, room: u32, leak_mult: f64, pass: f64) {
        let layer = &mut self.layers[field];
        let i = room as usize - 1;
        if i < layer.leak_mult.len() {
            layer.leak_mult[i] = leak_mult;
            layer.pass[i] = pass;
        }
    }

    /// `(leak multiplier, pass fraction)` for a room, as its boundary set it.
    pub fn boundary(&self, field: usize, room: u32) -> (f64, f64) {
        let layer = &self.layers[field];
        let i = room as usize - 1;
        (layer.leak_mult.get(i).copied().unwrap_or(1.0), layer.pass.get(i).copied().unwrap_or(0.0))
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
                self.revision += 1;
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
        self.revision += 1;
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

    /// Recompute every outdoor value: terms (in dependency order), then
    /// pushes, unless pinned. Expired pushes ease out and are dropped.
    pub fn update_ambient(&mut self, defs: &DefDb, clock: Clock) {
        let tick = clock.tick;
        self.last_clock = Some(clock);
        let mut vals: Vec<i64> = self.atmos.iter().map(|a| a.value).collect();
        for &f in &defs.ambient_order {
            let fd = &defs.fields[f];
            let a = &mut self.atmos[f];
            for p in a.pushes.iter_mut() {
                if !p.fading && p.until.is_some_and(|u| tick >= u) {
                    *p = Push { from: p.value(tick), to: 0, start: tick, until: None, fading: true, ..p.clone() };
                }
            }
            a.pushes.retain(|p| !(p.fading && p.value(tick) == 0 && tick >= p.start + p.ease));
            let v = match a.pin {
                Some(v) => v,
                None => {
                    let env = AmbEnv { year: clock.year, hour: clock.hour, tick, seed: clock.seed, vals: &vals };
                    let base = if fd.terms.is_empty() { terms::to_q(fd.base) } else { fd.terms.eval(&env) };
                    base + a.pushes.iter().map(|p| p.value(tick)).sum::<i64>()
                }
            };
            a.value = v;
            vals[f] = v;
            self.layers[f].ambient = (v / (Q / FIXED as i64)) as i32;
        }
    }

    /// Add or replace a named contribution to a field's outdoor value.
    pub fn push_ambient(
        &mut self,
        field: usize,
        key: &str,
        value: f64,
        tick: u64,
        hours: Option<f64>,
        ease_hours: f64,
    ) {
        let ticks = |h: f64| (h.max(0.0) * TICKS_PER_DAY as f64 / 24.0).round() as u64;
        let a = &mut self.atmos[field];
        let from = a.pushes.iter().find(|p| p.key == key).map_or(0, |p| p.value(tick));
        let push = Push {
            key: key.to_string(),
            from,
            to: terms::to_q(value),
            start: tick,
            ease: ticks(ease_hours),
            until: hours.map(|h| tick + ticks(h)),
            fading: false,
        };
        match a.pushes.iter_mut().find(|p| p.key == key) {
            Some(p) => *p = push,
            None => a.pushes.push(push),
        }
    }

    /// Ease a named contribution out over `ease_hours` and drop it.
    pub fn clear_ambient(&mut self, field: usize, key: &str, tick: u64, ease_hours: f64) {
        let ease = (ease_hours.max(0.0) * TICKS_PER_DAY as f64 / 24.0).round() as u64;
        if let Some(p) = self.atmos[field].pushes.iter_mut().find(|p| p.key == key) {
            *p = Push { from: p.value(tick), to: 0, start: tick, ease, until: None, fading: true, key: p.key.clone() };
        }
    }

    /// Each part of a field's outdoor value, as last computed: its terms by
    /// label, then pushes by key. A pin is reported alone.
    pub fn explain_ambient(&self, defs: &DefDb, field: usize) -> Vec<(String, f64)> {
        let a = &self.atmos[field];
        let clock = self.last_clock.unwrap_or(Clock { tick: 0, year: 0, hour: 0, seed: 0 });
        if let Some(v) = a.pin {
            return vec![("pinned".into(), terms::from_q(v))];
        }
        let vals: Vec<i64> = self.atmos.iter().map(|a| a.value).collect();
        let env = AmbEnv { year: clock.year, hour: clock.hour, tick: clock.tick, seed: clock.seed, vals: &vals };
        let fd = &defs.fields[field];
        let mut out: Vec<(String, f64)> = if fd.terms.is_empty() {
            vec![("base".into(), fd.base)]
        } else {
            fd.terms.explain(&env).into_iter().map(|(l, v)| (l, terms::from_q(v))).collect()
        };
        out.extend(a.pushes.iter().map(|p| (p.key.clone(), terms::from_q(p.value(clock.tick)))));
        out
    }

    /// Evaluate global terms (no per-cell inputs) against the outdoor values
    /// as last computed. The renderer uses it for sky tints.
    pub fn eval_global(&self, terms: &terms::Terms) -> f64 {
        let clock = self.last_clock.unwrap_or(Clock { tick: 0, year: 0, hour: 0, seed: 0 });
        let vals: Vec<i64> = self.atmos.iter().map(|a| a.value).collect();
        let env = AmbEnv { year: clock.year, hour: clock.hour, tick: clock.tick, seed: clock.seed, vals: &vals };
        terms::from_q(terms.eval(&env))
    }

    /// Call once per tick after rooms are current.
    pub fn update(&mut self, defs: &DefDb, map: &mut Map, clock: Clock) {
        let tick = clock.tick;
        if tick.is_multiple_of(AMBIENT_INTERVAL) {
            self.update_ambient(defs, clock);
        }
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
                let leak = fd.leak_per_hour * layer.leak_mult.get(r).copied().unwrap_or(1.0);
                let change = (ambient as f64 - v) * leak + heat / room.cells as f64;
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
            // Dark inside, except for what the boundary lets through.
            (IndoorMode::None, Some(r)) => {
                let pass = layer.pass.get(r.id as usize - 1).copied().unwrap_or(0.0);
                stamped + (layer.ambient as f64 * pass).round() as i32
            }
            // The room's value already includes what its emitters put in;
            // adding their local stamp too would count the same fire twice.
            (IndoorMode::Room, Some(r)) => layer.rooms.get(r.id as usize - 1).copied().unwrap_or(layer.ambient),
        }
    }

    /// Value at a cell, in the field's own units.
    pub fn value(&self, defs: &DefDb, map: &Map, field: usize, p: IVec) -> f64 {
        self.value_fixed(defs, map, field, p) as f64 / FIXED
    }

    /// Pin a field's outdoor value, overriding its terms and pushes, or
    /// unpin it with `None`. For tests and tools; mods push instead.
    pub fn set_ambient(&mut self, field: usize, v: Option<f64>) {
        let a = &mut self.atmos[field];
        a.pin = v.map(terms::to_q);
        if let Some(v) = a.pin {
            a.value = v;
            self.layers[field].ambient = (v / (Q / FIXED as i64)) as i32;
        }
    }

    pub fn ambient(&self, field: usize) -> f64 {
        terms::from_q(self.atmos[field].value)
    }

    pub fn emitter_count(&self) -> usize {
        self.emitters.len()
    }
}
