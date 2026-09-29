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
//!
//! A **stock** field is different: it stores a value per cell and changes
//! it by its `rate` terms, so it remembers (wet ground after rain). Each tick
//! works out one slice of the map, so every cell is worked out exactly once
//! a period, over the whole period, and the cost per tick is the map's
//! cells over the period's ticks, whatever else happens.

use crate::defs::{DefDb, DefId, FieldKind, IndoorMode, StockLevels};
use crate::map::{Map, NEIGHBORS8};
use crate::sky::{self, BodyState};
use crate::terms::{self, BodyOf, Env, Q};
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
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
        if self.ease == 0 || tick >= self.start.saturating_add(self.ease) {
            return self.to;
        }
        let t = tick.saturating_sub(self.start) as i64;
        match (self.to.checked_sub(self.from).and_then(|d| d.checked_mul(t)), i64::try_from(self.ease)) {
            (Some(moved), Ok(ease)) => self.from + moved / ease,
            // A script's value or ease past what i64 multiplies: the same
            // line, worked in i128, off the common path.
            _ => (self.from as i128 + (self.to as i128 - self.from as i128) * t as i128 / self.ease as i128) as i64,
        }
    }
}

/// Outdoor-value state for one field.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
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
    bodies: &'a [BodyState],
    /// Levels below the surface, times `Q`, for `below` terms.
    depth: i64,
}

impl Env for AmbEnv<'_> {
    fn year(&self) -> i64 {
        self.year
    }
    fn hour(&self) -> i64 {
        self.hour
    }
    fn depth(&self) -> i64 {
        self.depth
    }
    fn body(&self, body: usize, of: BodyOf) -> i64 {
        of.read(&self.bodies[body])
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

/// What a derived or stock field's terms see at one cell: `field`,
/// `terrain`, `near` and `sky` read there, and a stock field's own value
/// and base; everything else reads as outdoors does.
struct CellEnv<'a> {
    fields: &'a Fields,
    defs: &'a DefDb,
    map: &'a Map,
    p: IVec,
    clock: Clock,
    own: i64,
    base: i64,
}

impl Env for CellEnv<'_> {
    fn year(&self) -> i64 {
        self.clock.year
    }
    fn hour(&self) -> i64 {
        self.clock.hour
    }
    fn ambient(&self, f: usize) -> i64 {
        match self.p.z {
            z if z < 0 => self.fields.outdoor(f, z) as i64 * (Q / FIXED as i64),
            _ => self.fields.atmos[f].value,
        }
    }
    fn depth(&self) -> i64 {
        (-self.p.z).max(0) as i64 * Q
    }
    fn body(&self, body: usize, of: BodyOf) -> i64 {
        of.read(&self.fields.bodies[body])
    }
    fn field(&self, f: usize) -> i64 {
        self.fields.value_fixed(self.defs, self.map, f, self.p) as i64 * (Q / FIXED as i64)
    }
    fn tick(&self) -> u64 {
        self.clock.tick
    }
    fn seed(&self) -> u64 {
        self.clock.seed
    }
    fn terrain(&self, prop: usize) -> i64 {
        self.defs.terrain[self.map.terrain[self.map.idx(self.p)] as usize].props_q[prop]
    }
    fn near(&self, tag: usize) -> i64 {
        self.map.near(tag, self.map.idx(self.p)) as i64 * Q
    }
    // Only an enclosed room keeps the weather off. A cell merely within a
    // wall's or a cliff's roof span is open ground beside it, and gets rained on.
    fn sky(&self) -> i64 {
        if self.map.indoors(self.p) {
            0
        } else {
            Q
        }
    }
    fn own(&self) -> i64 {
        self.own
    }
    fn base(&self) -> i64 {
        self.base
    }
}

/// A cell's terms with the weather kept off: `sky` is 0.
struct Sheltered<'a>(CellEnv<'a>);

impl Env for Sheltered<'_> {
    fn year(&self) -> i64 {
        self.0.year()
    }
    fn hour(&self) -> i64 {
        self.0.hour()
    }
    fn ambient(&self, f: usize) -> i64 {
        self.0.ambient(f)
    }
    fn field(&self, f: usize) -> i64 {
        self.0.field(f)
    }
    fn tick(&self) -> u64 {
        self.0.tick()
    }
    fn seed(&self) -> u64 {
        self.0.seed()
    }
    fn terrain(&self, prop: usize) -> i64 {
        self.0.terrain(prop)
    }
    fn near(&self, tag: usize) -> i64 {
        self.0.near(tag)
    }
    fn body(&self, body: usize, of: BodyOf) -> i64 {
        self.0.body(body, of)
    }
    fn sky(&self) -> i64 {
        0
    }
}

/// How many cells a stock field keeps: the surface's, or every level's.
fn stock_cells(levels: StockLevels, map: &Map) -> usize {
    match levels {
        StockLevels::Surface => map.plane(),
        StockLevels::All => map.cells(),
    }
}

/// The clock, as terms see it.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
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

/// The part of `Fields` a save keeps.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SavedFields {
    pub atmos: Vec<Atmos>,
    pub ambient: Vec<i32>,
    /// Per field, each room's value, indexed by `room_ids`.
    pub rooms: Vec<Vec<i32>>,
    pub last_clock: Option<Clock>,
    /// Rooms were rebuilt (or are about to be) since the values were carried.
    pub pending_carry: bool,
    /// Each cell's room id, as the room values know it.
    pub room_ids: Vec<u32>,
    /// Per field, a stock field's value per cell in `Q` units; empty for
    /// other fields, and for one not worked out yet.
    #[serde(default)]
    pub stock: Vec<Vec<i32>>,
    /// Per field, the outdoor value on each level below the surface, as
    /// last worked out: the next ambient update is up to
    /// `AMBIENT_INTERVAL` ticks away. Empty in older saves.
    #[serde(default)]
    pub below: Vec<Vec<i32>>,
}

pub struct Layer {
    /// Emitter contributions per cell.
    pub stamped: Vec<i32>,
    /// Open-sky value.
    pub ambient: i32,
    /// The outdoor value below the surface, a level at a time from -1
    /// (DESIGN.md §6d): the field's `below` terms, or 0.
    pub below: Vec<i32>,
    /// Per-room value for `indoor = "room"` fields, indexed by room id - 1.
    pub rooms: Vec<i32>,
    /// Per-room multiplier on the field's leak, from what encloses the room.
    /// 1.0 for a boundary that says nothing.
    pub leak_mult: Vec<f64>,
    /// Per-room fraction of the outdoor value that gets in through the
    /// boundary, for fields that are otherwise dark indoors.
    pub pass: Vec<f64>,
    /// Shelter fields: each cell's exposure in percent, and the wind octant
    /// and map revision it was worked out for (`World::update_shelter`).
    pub exposure: Vec<u8>,
    pub exposure_for: Option<(u8, u64)>,
    /// Stock fields: each cell's value in `Q` units, on the surface's plane
    /// or on every level's. Empty until the first update works it out from
    /// the `init` terms.
    pub stock: Vec<i32>,
    /// Stock fields with `move_cost`: the extra cost each cell adds to the
    /// map, in 10% steps. Empty until worked out from the values, which a
    /// load does afresh, so it is never saved.
    pub move_bucket: Vec<u8>,
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
    /// Scratch for a stock slice's new values, `(field, cell, value)`, so a
    /// tick's reads all see the values from before it.
    stock_next: Vec<(u32, u32, i32)>,
    /// Stock cells a script set since the last update, `(field, cell)`,
    /// for the move costs to catch up on.
    stock_touched: Vec<(u32, u32)>,
    /// Cells whose move cost changed bucket, for tests and the profiler.
    pub move_changes: u64,
    /// Where each sky body is, worked out with the outdoor values, in
    /// `defs.sky_bodies` order.
    bodies: Vec<BodyState>,
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
                    below: Vec::new(),
                    rooms: Vec::new(),
                    leak_mult: Vec::new(),
                    pass: Vec::new(),
                    exposure: Vec::new(),
                    exposure_for: None,
                    stock: Vec::new(),
                    move_bucket: Vec::new(),
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
            bodies: sky::states(defs, 0),
            stock_next: Vec::new(),
            stock_touched: Vec::new(),
            move_changes: 0,
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
        self.bodies = sky::states(defs, tick);
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
                    let env = AmbEnv {
                        year: clock.year,
                        hour: clock.hour,
                        tick,
                        seed: clock.seed,
                        vals: &vals,
                        bodies: &self.bodies,
                        depth: 0,
                    };
                    let base = if fd.terms.is_empty() { terms::to_q(fd.base) } else { fd.terms.eval(&env) };
                    base + a.pushes.iter().map(|p| p.value(tick)).sum::<i64>()
                }
            };
            a.value = v;
            vals[f] = v;
            self.layers[f].ambient = (v / (Q / FIXED as i64)) as i32;
            // Below, what the ground keeps: no pushes reach it, so a cold
            // snap stays on the surface.
            let below = &mut self.layers[f].below;
            below.clear();
            for depth in 1..=defs.depth() as i64 {
                let v = match fd.below_terms.terms.is_empty() {
                    true => 0,
                    false => {
                        let env = AmbEnv {
                            year: clock.year,
                            hour: clock.hour,
                            tick,
                            seed: clock.seed,
                            vals: &vals,
                            bodies: &self.bodies,
                            depth: depth * Q,
                        };
                        fd.below_terms.eval(&env)
                    }
                };
                below.push((v / (Q / FIXED as i64)) as i32);
            }
        }
    }

    /// Field `f`'s outdoor value on level `z`, in hundredths: the sky's on
    /// and above the surface, the ground's below it.
    pub fn outdoor(&self, f: usize, z: i32) -> i32 {
        let l = &self.layers[f];
        match z {
            z if z < 0 => l.below.get((-z - 1) as usize).copied().unwrap_or(0),
            _ => l.ambient,
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
            until: hours.map(|h| tick.saturating_add(ticks(h))),
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
        let env = AmbEnv {
            year: clock.year,
            hour: clock.hour,
            tick: clock.tick,
            seed: clock.seed,
            vals: &vals,
            bodies: &self.bodies,
            depth: 0,
        };
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
        let env = AmbEnv {
            year: clock.year,
            hour: clock.hour,
            tick: clock.tick,
            seed: clock.seed,
            vals: &vals,
            bodies: &self.bodies,
            depth: 0,
        };
        terms::from_q(terms.eval(&env))
    }

    /// Call once per tick after rooms are current.
    pub fn update(&mut self, defs: &DefDb, map: &mut Map, clock: Clock) {
        let tick = clock.tick;
        if tick.is_multiple_of(AMBIENT_INTERVAL) {
            self.update_ambient(defs, clock);
        }
        map.ensure_near();
        let changed = map.take_changed_cells();
        if !changed.is_empty() {
            self.restamp_near(map, &changed);
        }
        // Mud follows the terrain under it: a cell dug or filled is costed
        // again now, not when its slice next comes round.
        let terrain_moved = std::mem::take(&mut self.stock_touched)
            .into_iter()
            .chain(defs.move_fields.iter().flat_map(|&f| changed.iter().map(move |&c| (f as u32, c))))
            .collect::<Vec<_>>();
        if map.room_rebuilds != self.seen_rebuilds {
            self.carry_rooms_over(defs, map);
            self.seen_rebuilds = map.room_rebuilds;
        }
        if tick.is_multiple_of(ROOM_INTERVAL) {
            self.step_rooms(defs, map);
        }
        self.step_stock(defs, map, clock);
        self.update_move_costs(defs, map, &terrain_moved);
    }

    /// Snow and mud: the map's extra move cost from each `move_cost` field,
    /// in 10% steps, changed only where a cell's step changed. It is worked
    /// out for this tick's stock slice and the cells in `also`; for every
    /// cell when a field has none yet (a new map, a load). Either way each
    /// cell's cost is its current value's, so a load walks as the live game.
    fn update_move_costs(&mut self, defs: &DefDb, map: &mut Map, also: &[(u32, u32)]) {
        let next = std::mem::take(&mut self.stock_next);
        for &f in &defs.move_fields {
            let n = self.layers[f].stock.len();
            if self.layers[f].move_bucket.len() != n {
                self.layers[f].move_bucket = vec![0; n];
                for i in 0..n {
                    self.cost_cell(defs, map, f, i);
                }
            }
        }
        for &(f, i, _) in &next {
            if defs.fields[f as usize].move_curve.is_some() {
                self.cost_cell(defs, map, f as usize, i as usize);
            }
        }
        for &(f, i) in also {
            if (i as usize) < self.layers[f as usize].move_bucket.len() {
                self.cost_cell(defs, map, f as usize, i as usize);
            }
        }
        self.stock_next = next;
    }

    /// Work out one cell's move cost from one field, and move the map's
    /// extra cost by the change.
    fn cost_cell(&mut self, defs: &DefDb, map: &mut Map, f: usize, i: usize) {
        let fd = &defs.fields[f];
        let Some(curve) = &fd.move_curve else { return };
        let layer = &mut self.layers[f];
        let mut pct = curve.eval(layer.stock[i] as i64);
        if let Some(prop) = fd.move_by_r {
            pct = pct * defs.terrain[map.terrain[i] as usize].props_q[prop] / Q;
        }
        let step = ((pct.max(0) + 5 * Q) / (10 * Q)).min(u8::MAX as i64) as u8;
        let was = layer.move_bucket[i];
        if step != was {
            layer.move_bucket[i] = step;
            map.add_extra_cost(i, (step as i32 - was as i32) * 10);
            self.move_changes += 1;
        }
    }

    /// Work out every stock field that hasn't been from its `init` terms:
    /// on the first update after the map is made, or after a load that
    /// didn't have it (or kept it for other levels).
    pub fn ensure_stock(&mut self, defs: &DefDb, map: &Map) {
        let clock = self.last_clock.unwrap_or(Clock { tick: 0, year: 0, hour: 0, seed: 0 });
        for &f in &defs.stock_fields {
            let fd = &defs.fields[f];
            let n = stock_cells(fd.levels, map);
            if self.layers[f].stock.len() == n {
                continue;
            }
            let (lo, hi) = (terms::to_q(fd.range[0]), terms::to_q(fd.range[1]));
            // The surface is the first plane, so its cells index the same.
            let v: Vec<i32> = (0..n)
                .map(|i| {
                    let env = CellEnv { fields: self, defs, map, p: map.pos(i), clock, own: 0, base: 0 };
                    fd.init_terms.eval(&env).clamp(lo, hi) as i32
                })
                .collect();
            self.layers[f].stock = v;
        }
    }

    /// This tick's slice of every stock field: each cell once a period, by
    /// its rate (terms plus emitters) over the whole period, clamped to the
    /// field's range. Every read in a tick sees the values from before it.
    pub fn step_stock(&mut self, defs: &DefDb, map: &Map, clock: Clock) {
        if defs.stock_fields.is_empty() {
            return;
        }
        self.ensure_stock(defs, map);
        let mut next = std::mem::take(&mut self.stock_next);
        next.clear();
        for &f in &defs.stock_fields {
            let fd = &defs.fields[f];
            // No rate terms: it holds what scripts and map generation put
            // there (ore), and costs nothing a tick.
            if fd.rate_terms.is_empty() {
                continue;
            }
            let n = stock_cells(fd.levels, map) as u64;
            let k = clock.tick % fd.period;
            let (a, b) = ((k * n / fd.period) as usize, ((k + 1) * n / fd.period) as usize);
            let (lo, hi) = (terms::to_q(fd.range[0]), terms::to_q(fd.range[1]));
            let layer = &self.layers[f];
            for i in a..b {
                let own = layer.stock[i] as i64;
                let mut env = CellEnv { fields: self, defs, map, p: map.pos(i), clock, own, base: 0 };
                if !fd.settle_terms.is_empty() {
                    env.base = fd.settle_terms.eval(&env);
                }
                // Per hour, times the period's hours.
                let rate = fd.rate_terms.eval(&env) + layer.stamped[i] as i64 * (Q / FIXED as i64);
                let v = own + rate * fd.period as i64 * 24 / TICKS_PER_DAY as i64;
                next.push((f as u32, i as u32, v.clamp(lo, hi) as i32));
            }
        }
        for &(f, i, v) in &next {
            self.layers[f as usize].stock[i as usize] = v;
        }
        self.stock_next = next;
    }

    /// Terms read at a cell as a derived field's are: `field`, `terrain`,
    /// `near` and `sky` there, in `Q` units. Plants grow by them.
    pub fn eval_at(&self, defs: &DefDb, map: &Map, terms: &terms::Terms, p: IVec) -> i64 {
        let clock = self.last_clock.unwrap_or(Clock { tick: 0, year: 0, hour: 0, seed: 0 });
        terms.eval(&CellEnv { fields: self, defs, map, p, clock, own: 0, base: 0 })
    }

    /// `eval_at`, out of the weather: `sky` reads 0, as under a roof. What
    /// lies in a sheltering store is read this way.
    pub fn eval_sheltered(&self, defs: &DefDb, map: &Map, terms: &terms::Terms, p: IVec) -> i64 {
        let clock = self.last_clock.unwrap_or(Clock { tick: 0, year: 0, hour: 0, seed: 0 });
        terms.eval(&Sheltered(CellEnv { fields: self, defs, map, p, clock, own: 0, base: 0 }))
    }

    /// Set a stock field at a cell (`add`: add to it), clamped to its range.
    /// The new value, in the field's units; `None` on a level it isn't kept on.
    pub fn set_stock(&mut self, defs: &DefDb, map: &Map, field: usize, p: IVec, v: f64, add: bool) -> Option<f64> {
        self.ensure_stock(defs, map);
        let fd = &defs.fields[field];
        let i = map.idx(p);
        let cell = self.layers[field].stock.get_mut(i)?;
        let now = if add { (*cell as i64).saturating_add(terms::to_q(v)) } else { terms::to_q(v) };
        *cell = now.clamp(terms::to_q(fd.range[0]), terms::to_q(fd.range[1])) as i32;
        let v = terms::from_q(*cell as i64);
        self.stock_touched.push((field as u32, i as u32));
        Some(v)
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
            for i in 0..map.cells() {
                let (new_id, old_id) = map.room_ids(i);
                if new_id == 0 {
                    continue;
                }
                let here = match map.pos(i).z {
                    z if z < 0 => layer.below.get((-z - 1) as usize).copied().unwrap_or(0),
                    _ => layer.ambient,
                };
                let before = old.get(old_id.wrapping_sub(1) as usize).copied().unwrap_or(here);
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
            let leak_now = match fd.leak_terms.terms.is_empty() {
                true => fd.leak_per_hour,
                false => self.eval_global(&fd.leak_terms).max(0.0),
            };
            let layer = &mut self.layers[fi];
            layer.rooms.resize(n, layer.ambient);
            // Summed as integers: the emitter list is in the order things were
            // built, which a load can't reproduce, and float sums depend on it.
            let mut heat = vec![0i64; n];
            for e in self.emitters.iter().filter(|e| e.field == fi) {
                let Some(r) = map.room_at(e.pos) else { continue };
                let now = layer.rooms[r.id as usize - 1];
                // A capped emitter stops pushing once the room reaches its cap.
                let spent = e.cap.is_some_and(|c| if e.amount >= 0 { now >= c } else { now <= c });
                if !spent {
                    heat[r.id as usize - 1] += e.amount as i64;
                }
            }
            for (r, (value, heat)) in layer.rooms.iter_mut().zip(&heat).enumerate() {
                let room = map.room_by_id(r as u32 + 1);
                // A room leaks toward the outdoors of its own level.
                let ambient = match room.level {
                    z if z < 0 => layer.below.get((-z - 1) as usize).copied().unwrap_or(0),
                    _ => layer.ambient,
                };
                if !room.enclosed() {
                    *value = ambient;
                    continue;
                }
                let v = *value as f64;
                let leak = leak_now * layer.leak_mult.get(r).copied().unwrap_or(1.0);
                let change = (ambient as f64 - v) * leak + *heat as f64 * fd.room_gain / room.cells as f64;
                *value = (v + change * hours).round() as i32;
            }
        }
    }

    /// What a save keeps (DESIGN.md §7a). Stamps and room boundaries are
    /// rebuilt from the map. Room values are indexed by room id, so the save
    /// keeps the grid of ids they belong to (`Map::carry_from`).
    pub fn saved(&self, map: &Map) -> SavedFields {
        SavedFields {
            atmos: self.atmos.clone(),
            ambient: self.layers.iter().map(|l| l.ambient).collect(),
            rooms: self.layers.iter().map(|l| l.rooms.clone()).collect(),
            last_clock: self.last_clock,
            pending_carry: map.room_rebuilds != self.seen_rebuilds || map.rooms_dirty(),
            room_ids: map.carry_from(self.seen_rebuilds).to_vec(),
            stock: self.layers.iter().map(|l| l.stock.clone()).collect(),
            below: self.layers.iter().map(|l| l.below.clone()).collect(),
        }
    }

    /// Put back what `saved` kept, once the map has its things and rooms.
    /// `map_changed`: the map isn't the one the values were saved on, so
    /// they are carried over to its rooms cell by cell.
    pub fn restore(&mut self, defs: &DefDb, map: &mut Map, s: SavedFields, map_changed: bool) {
        self.atmos = s.atmos;
        for (l, (ambient, rooms)) in self.layers.iter_mut().zip(s.ambient.into_iter().zip(s.rooms)) {
            l.ambient = ambient;
            l.rooms = rooms;
        }
        // A stock field the save lacks, or kept for other cells, is worked
        // out afresh from its init terms (`ensure_stock`).
        for (l, stock) in self.layers.iter_mut().zip(s.stock) {
            l.stock = stock;
        }
        for (l, below) in self.layers.iter_mut().zip(s.below) {
            l.below = below;
        }
        self.last_clock = s.last_clock;
        // Bodies aren't saved: they follow from the tick they were last
        // worked out at.
        self.bodies = sky::states(defs, s.last_clock.map_or(0, |c| c.tick));
        if s.pending_carry || map_changed {
            // The live game carries room values over to the new rooms on its
            // next update, from these ids; so will this one.
            map.set_prev_rooms(s.room_ids);
        } else {
            self.seen_rebuilds = map.room_rebuilds;
        }
    }

    /// Where each sky body is, as last worked out, in `defs.sky_bodies`
    /// order.
    pub fn bodies(&self) -> &[BodyState] {
        &self.bodies
    }

    /// Value at a cell, in hundredths.
    pub fn value_fixed(&self, defs: &DefDb, map: &Map, field: usize, p: IVec) -> i32 {
        if !map.inb(p) {
            return 0;
        }
        let layer = &self.layers[field];
        match defs.fields[field].kind {
            FieldKind::Derived => return self.derived_at(defs, map, field, p),
            // To the nearest hundredth: a value settling on 0.4 from below
            // reads 0.4, not 0.39.
            FieldKind::Stock => {
                let step = Q / FIXED as i64;
                return layer.stock.get(map.idx(p)).map_or(0, |&v| (v as i64 + step / 2).div_euclid(step) as i32);
            }
            _ => {}
        }
        let indoors = map.room_at(p).filter(|r| r.enclosed());
        if defs.fields[field].kind == FieldKind::Shelter {
            // No wind reaches below the surface (DESIGN.md §6d).
            let open = match p.z {
                0 => layer.exposure.get(map.idx(p)).copied().unwrap_or(100) as i32,
                _ => 0,
            };
            return if indoors.is_some() { 0 } else { open * FIXED as i32 };
        }
        let stamped = layer.stamped[map.idx(p)];
        let outdoor = self.outdoor(field, p.z);
        match (defs.fields[field].indoor, indoors) {
            (_, None) | (IndoorMode::Outdoor, _) => outdoor + stamped,
            // Dark inside, except for what the boundary lets through.
            (IndoorMode::None, Some(r)) => {
                let pass = layer.pass.get(r.id as usize - 1).copied().unwrap_or(0.0);
                stamped + (outdoor as f64 * pass).round() as i32
            }
            // The room's value already includes what its emitters put in;
            // adding their local stamp too would count the same fire twice.
            (IndoorMode::Room, Some(r)) => layer.rooms.get(r.id as usize - 1).copied().unwrap_or(outdoor),
        }
    }

    /// A derived field at a cell: its terms read there, plus whatever is
    /// pushed onto its outdoor value. A pin holds everywhere.
    fn derived_at(&self, defs: &DefDb, map: &Map, field: usize, p: IVec) -> i32 {
        let a = &self.atmos[field];
        let q = match a.pin {
            Some(v) => v,
            None => {
                let clock = self.last_clock.unwrap_or(Clock { tick: 0, year: 0, hour: 0, seed: 0 });
                let env = CellEnv { fields: self, defs, map, p, clock, own: 0, base: 0 };
                defs.fields[field].terms.eval(&env) + a.pushes.iter().map(|p| p.value(clock.tick)).sum::<i64>()
            }
        };
        (q / (Q / FIXED as i64)) as i32
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

    /// The emitters of `field`: whose they are, where, how strong at the
    /// source (in the field's units) and how far they reach. For what draws
    /// the field, such as the renderer's firelight; the sim reads stamps.
    pub fn emitters_of(&self, field: usize) -> impl Iterator<Item = (Entity, IVec, f64, u32)> + '_ {
        self.emitters
            .iter()
            .filter(move |e| e.field == field)
            .map(|e| (e.entity, e.pos, e.amount as f64 / FIXED, e.radius))
    }
}
