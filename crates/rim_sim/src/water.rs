//! Water in basins (DESIGN.md §6d).
//!
//! A basin is a connected space water can stand in on one level: below the
//! surface every open cell (dug out, not walled off), on and above it only
//! air, so a pit or a trench. Basins are derived from the map, rebuilt a
//! level at a time when what water can stand in there changes, and only
//! their water is saved. Depth is in sevenths of a cell, the same over a
//! basin's wet cells; the wet front grows a ring a tick from where the water
//! came in.
//!
//! Water comes from terrain that pours (the water table: a river is never
//! simulated and never runs out) through each face beside or above a basin,
//! and from rock that seeps. It falls first, through air and down stairs,
//! into the basin below until that one is full. It never climbs.

use crate::defs::{DefDb, DefId};
use crate::map::Map;
use crate::IVec;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A full cell, in sevenths.
pub const FULL: u32 = 7;

/// What a hole passes down in a tick: a cell of water.
const HOLE_RATE: u64 = FULL as u64;

#[derive(Clone, Debug, Default)]
pub struct Basin {
    /// Its cells, in the order water reaches them.
    pub cells: Vec<u32>,
    /// Cells wet once the front has grown `k + 1` rings.
    rings: Vec<u32>,
    /// Water, in sevenths of a cell.
    pub volume: u64,
    /// Rings the front has grown.
    front: u32,
    /// Sevenths a tick from water that never runs out.
    pours: u64,
    /// Sevenths a day from rock that seeps, and what is owed of it.
    seeps: u64,
    owed: u64,
    /// The cells below its holes (air, stairs down): where it falls.
    holes: Vec<u32>,
    /// Where a source first touches it: the basin's cell and the source's
    /// terrain, for the breach event.
    source: Option<(u32, DefId)>,
    /// The step cost and wet cells last put on the map (`Water::costs`).
    applied: Option<(u16, u32)>,
    /// Its volume at the last `costs`, and whether it had risen since.
    last_volume: u64,
    rising: bool,
    /// Depth and wet cells when the level's revision last moved for it.
    shown: (u32, u32),
}

impl Basin {
    pub fn capacity(&self) -> u64 {
        self.cells.len() as u64 * FULL as u64
    }

    /// Cells the water has reached.
    pub fn wet(&self) -> u32 {
        match self.front {
            0 => 0,
            f => self.rings[f as usize - 1],
        }
    }

    /// Depth over the wet cells, in sevenths: any water at all wets them.
    pub fn depth(&self) -> u32 {
        match self.wet() as u64 {
            0 => 0,
            wet => self.volume.div_ceil(wet).min(FULL as u64) as u32,
        }
    }

    fn room(&self) -> u64 {
        self.capacity().saturating_sub(self.volume)
    }

    fn sourced(&self) -> bool {
        self.pours > 0 || self.seeps > 0
    }
}

/// One level's basins, and the revisions they were built at: the level's
/// water revision, and the terrain above it, which its sources and inlets
/// read.
#[derive(Clone, Debug, Default)]
struct Level {
    basins: Vec<Basin>,
    built: Option<(u64, u64)>,
    /// Bumped whenever what its water looks like changes: a rebuild, or a
    /// basin's depth or wet cells. What a renderer keys on.
    rev: u64,
}

/// A basin's water in a save: named by its lowest cell, since basins are
/// rebuilt from the map on load.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedBasin {
    pub cell: u32,
    pub volume: u64,
    pub owed: u64,
    pub wet: u32,
    /// Its volume at the last `Water::costs`, and whether it had risen:
    /// what the next pass and the escape from rising water read. None in
    /// older saves.
    #[serde(default)]
    pub last_volume: u64,
    #[serde(default)]
    pub rising: bool,
}

/// What the water last put on the map (`Water::costs`), which moves only
/// every `WATER_EVERY` ticks: each cell's step cost where it has one, and
/// the cells of rebuilt basins still to be cleared.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SavedCosts {
    pub cells: Vec<(u32, u16)>,
    pub stale: Vec<u32>,
}

/// Water broke into a basin that had none coming: where, and from what.
#[derive(Clone, Debug, PartialEq)]
pub struct Breach {
    pub at: IVec,
    pub source: DefId,
}

#[derive(Clone, Debug, Default)]
pub struct Water {
    /// By level, lowest first.
    levels: Vec<Level>,
    lowest: i32,
    /// Per cell: its basin's index on its level, plus one; 0 is none.
    basin_of: Vec<u32>,
    /// Per cell: its place in its basin's `cells`.
    place: Vec<u32>,
    /// Built once: the first build finds what was always there, which is
    /// no breach.
    primed: bool,
    /// Per cell, the last walk that reached it: a visited mark no walk has
    /// to clear.
    mark: Vec<u32>,
    walk: u32,
    /// Cells of basins a rebuild took away, whose cost on the map is stale.
    stale: Vec<u32>,
}

impl Water {
    pub fn new(map: &Map) -> Self {
        let levels = map.levels();
        Water {
            levels: vec![Level::default(); levels.clone().count()],
            lowest: *levels.start(),
            basin_of: vec![0; map.cells()],
            place: vec![0; map.cells()],
            primed: false,
            mark: vec![0; map.cells()],
            walk: 0,
            stale: Vec::new(),
        }
    }

    fn level(&self, z: i32) -> usize {
        (z - self.lowest) as usize
    }

    /// Depth at `p`, in sevenths of a cell.
    pub fn depth(&self, map: &Map, p: IVec) -> u32 {
        if !map.inb(p) {
            return 0;
        }
        let i = map.idx(p);
        let Some(b) = self.basin_at(p.z, i) else { return 0 };
        if self.place[i] < b.wet() {
            b.depth()
        } else {
            0
        }
    }

    fn basin_at(&self, z: i32, i: usize) -> Option<&Basin> {
        let b = self.basin_of[i].checked_sub(1)?;
        self.levels[self.level(z)].basins.get(b as usize)
    }

    /// Level `z`'s basins, in id order.
    pub fn basins(&self, z: i32) -> &[Basin] {
        &self.levels[self.level(z)].basins
    }

    /// Rebuild the levels whose water revision moved, carrying their water
    /// over. Returns breaches, unless `quiet` (a load, the first build).
    pub fn update(&mut self, map: &Map, defs: &DefDb, quiet: bool) -> Vec<Breach> {
        let quiet = quiet || !self.primed;
        self.primed = true;
        let mut out = Vec::new();
        for z in map.levels().rev() {
            let now = (map.water_revision(z), map.ground_revision(z + 1));
            if self.levels[self.level(z)].built != Some(now) {
                self.rebuild(map, defs, z, quiet, &mut out);
                let k = self.level(z);
                self.levels[k].built = Some(now);
            }
        }
        out
    }

    /// Can water stand in cell `i` of level `z`?
    fn open(map: &Map, defs: &DefDb, z: i32, i: usize) -> bool {
        let t = &defs.terrain[map.terrain[i] as usize];
        if z >= 0 {
            t.air
        } else {
            t.solid.is_none() && t.pours == 0 && !map.blocks_water(i)
        }
    }

    fn rebuild(&mut self, map: &Map, defs: &DefDb, z: i32, quiet: bool, breaches: &mut Vec<Breach>) {
        let k = self.level(z);
        let start = map.idx(IVec::at(0, 0, z));
        let range = start..start + map.plane();
        let old = std::mem::take(&mut self.levels[k].basins);
        self.stale.extend(old.iter().flat_map(|b| b.cells.iter().copied()));
        let old_of = self.basin_of[range.clone()].to_vec();
        let old_place = self.place[range.clone()].to_vec();
        self.basin_of[range.clone()].fill(0);
        let (w, h) = (map.w, map.h);
        let near = move |p: IVec| {
            [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .into_iter()
                .map(move |(dx, dy)| p.offset(dx, dy))
                .filter(move |q| q.x >= 0 && q.y >= 0 && q.x < w && q.y < h)
        };
        let above = |p: IVec| map.levels().contains(&(p.z + 1)).then(|| map.idx(IVec::at(p.x, p.y, p.z + 1)));
        let below = |p: IVec| map.levels().contains(&(p.z - 1)).then(|| IVec::at(p.x, p.y, p.z - 1));
        let terrain = |i: usize| &defs.terrain[map.terrain[i] as usize];

        let mut basins = Vec::new();
        let mut queue = VecDeque::new();
        // On and above the surface only air holds water: walk the air, not
        // the plane.
        let candidates: Vec<usize> = match z >= 0 {
            true => map.air_cells(z).iter().map(|&i| i as usize).collect(),
            false => range.clone().collect(),
        };
        for i in candidates {
            if self.basin_of[i] != 0 || !Self::open(map, defs, z, i) {
                continue;
            }
            // Flood the basin, in index order of discovery.
            let id = basins.len() as u32 + 1;
            let mut cells = vec![i as u32];
            self.basin_of[i] = id;
            queue.push_back(i);
            while let Some(c) = queue.pop_front() {
                for q in near(map.pos(c)) {
                    let j = map.idx(q);
                    if self.basin_of[j] == 0 && Self::open(map, defs, z, j) {
                        self.basin_of[j] = id;
                        cells.push(j as u32);
                        queue.push_back(j);
                    }
                }
            }
            cells.sort_unstable();
            let mut b = Basin::default();
            let mut inlets = Vec::new();
            for &c in &cells {
                let c = c as usize;
                let p = map.pos(c);
                let mut inlet = false;
                // Faces: water beside it or over it, rock that seeps.
                let faces = near(p).map(|q| map.idx(q)).chain(above(p));
                for f in faces {
                    let t = terrain(f);
                    let (pours, seeps) = (t.pours as u64, t.solid.as_ref().map_or(0, |s| s.seeps) as u64);
                    if pours + seeps > 0 {
                        b.pours += pours;
                        b.seeps += seeps;
                        b.source.get_or_insert((c as u32, map.terrain[f]));
                        inlet = true;
                    }
                }
                // Water from above comes in under air, or down stairs.
                let from_above =
                    above(p).is_some_and(|a| terrain(a).air) || map.portal_at(c).is_some_and(|pt| pt.bottom == p);
                inlet |= from_above;
                if inlet {
                    inlets.push(c as u32);
                }
                // Holes: it falls through air, and down stairs.
                if let Some(q) = below(p) {
                    let j = map.idx(q);
                    let stairs = map.portal_at(c).is_some_and(|pt| pt.top == p);
                    if (terrain(c).air || stairs) && Self::open(map, defs, q.z, j) {
                        b.holes.push(j as u32);
                    }
                }
            }
            // The front's rings: out from the inlets, or from its first cell.
            if inlets.is_empty() {
                inlets.push(cells[0]);
            }
            self.walk += 1;
            let walk = self.walk;
            for &c in &inlets {
                self.mark[c as usize] = walk;
            }
            // Ring by ring: each is the cells next to the last, in the
            // order they were found, so the order is the same every build.
            let mut order = inlets;
            order.reserve(cells.len());
            let mut from = 0;
            while from < order.len() {
                let to = order.len();
                b.rings.push(to as u32);
                for n in from..to {
                    for q in near(map.pos(order[n] as usize)) {
                        let j = map.idx(q);
                        if self.basin_of[j] == id && self.mark[j] != walk {
                            self.mark[j] = walk;
                            order.push(j as u32);
                        }
                    }
                }
                from = to;
            }
            for (n, &c) in order.iter().enumerate() {
                self.place[c as usize] = n as u32;
            }
            b.cells = order;
            basins.push(b);
        }

        // Carry the water over: each old basin's goes to the new ones its
        // cells are in, by how many, the remainder to the largest share.
        // Per old basin, the new ones its cells went to: (new, cells, wet
        // cells), in new-basin order.
        let mut went: Vec<Vec<(u32, u32, u32)>> = vec![Vec::new(); old.len()];
        for (n, nb) in basins.iter().enumerate() {
            let mut parts: Vec<(u32, u32, u32)> = Vec::new();
            for &c in &nb.cells {
                let off = c as usize - start;
                let Some(o) = old_of[off].checked_sub(1) else { continue };
                let wet = (old_place[off] < old[o as usize].wet()) as u32;
                match parts.iter_mut().find(|p| p.0 == o) {
                    Some(p) => {
                        p.1 += 1;
                        p.2 += wet;
                    }
                    None => parts.push((o, 1, wet)),
                }
            }
            for (o, cells, wet) in parts {
                went[o as usize].push((n as u32, cells, wet));
            }
        }
        let mut wet = vec![0u32; basins.len()];
        let mut was_sourced = vec![false; basins.len()];
        for (ob, parts) in old.iter().zip(&went) {
            let total: u64 = parts.iter().map(|p| p.1 as u64).sum();
            if total == 0 {
                continue;
            }
            let mut given = 0;
            for &(n, cells, wt) in parts {
                let nb = &mut basins[n as usize];
                let share = ob.volume * cells as u64 / total;
                nb.volume += share;
                given += share;
                wet[n as usize] += wt;
                was_sourced[n as usize] |= ob.sourced();
            }
            let most = parts.iter().max_by_key(|p| (p.1, std::cmp::Reverse(p.0))).expect("parts").0;
            let nb = &mut basins[most as usize];
            nb.volume += ob.volume - given;
            nb.owed += ob.owed;
        }
        for (n, b) in basins.iter_mut().enumerate() {
            b.volume = b.volume.min(b.capacity());
            // At least as far as the water had spread, and wet if any.
            let reached = b.rings.partition_point(|&r| r < wet[n]) as u32;
            b.front = (reached + (wet[n] > 0) as u32).min(b.rings.len() as u32);
            if b.volume > 0 && b.front == 0 {
                b.front = 1;
            }
            if !quiet && b.sourced() && !was_sourced[n] {
                if let Some((c, t)) = b.source {
                    breaches.push(Breach { at: map.pos(c as usize), source: t });
                }
            }
        }
        self.levels[k].basins = basins;
        self.levels[k].rev += 1;
    }

    /// One tick: sources fill their basins, then water falls, a level at a
    /// time from the top, so what reaches a level can fall on in the same
    /// tick; fronts grow.
    pub fn step(&mut self, ticks_per_day: u64) {
        for k in (0..self.levels.len()).rev() {
            for b in &mut self.levels[k].basins {
                if b.pours > 0 {
                    b.volume = (b.volume + b.pours).min(b.capacity());
                }
                if b.seeps > 0 && b.room() > 0 {
                    b.owed += b.seeps;
                    b.volume = (b.volume + b.owed / ticks_per_day).min(b.capacity());
                    b.owed %= ticks_per_day;
                }
            }
            if k > 0 {
                let (lower, upper) = self.levels.split_at_mut(k);
                let (here, under) = (&mut upper[0].basins, &mut lower[k - 1].basins);
                for b in here.iter_mut().filter(|b| b.volume > 0 && !b.holes.is_empty()) {
                    for &j in &b.holes {
                        let Some(t) = self.basin_of[j as usize].checked_sub(1) else { continue };
                        let t = &mut under[t as usize];
                        let m = b.volume.min(HOLE_RATE).min(t.room());
                        b.volume -= m;
                        t.volume += m;
                        if b.volume == 0 {
                            break;
                        }
                    }
                }
            }
            let level = &mut self.levels[k];
            for b in &mut level.basins {
                if b.volume > 0 && (b.front as usize) < b.rings.len() {
                    b.front += 1;
                }
                let now = (b.depth(), b.wet());
                if now != b.shown {
                    b.shown = now;
                    level.rev += 1;
                }
            }
        }
    }

    /// What level `z`'s water looks like changed when this moved.
    pub fn revision(&self, z: i32) -> u64 {
        match self.levels.get((z - self.lowest) as usize) {
            Some(l) => l.rev,
            None => 0,
        }
    }

    /// Is the water at `p` rising: more than at the last `costs`?
    pub fn rising(&self, map: &Map, p: IVec) -> bool {
        map.inb(p) && self.basin_at(p.z, map.idx(p)).is_some_and(|b| b.rising)
    }

    /// What the water adds to crossing each cell, where that changed since
    /// the last call: (cell, cost) in `out`, stale cells first. `cost_of`
    /// turns a depth into a step's cost. Also notes which basins rose.
    pub fn costs(&mut self, cost_of: impl Fn(u32) -> u16, out: &mut Vec<(u32, u16)>) {
        out.extend(self.stale.drain(..).map(|c| (c, 0)));
        for b in self.levels.iter_mut().flat_map(|l| &mut l.basins) {
            b.rising = b.volume > b.last_volume;
            b.last_volume = b.volume;
            let now = (cost_of(b.depth()), b.wet());
            if b.applied == Some(now) {
                continue;
            }
            b.applied = Some(now);
            let (cost, wet) = now;
            out.extend(b.cells.iter().enumerate().map(|(n, &c)| (c, if (n as u32) < wet { cost } else { 0 })));
        }
    }

    /// The water to save: every basin holding some.
    pub fn saved(&self) -> Vec<SavedBasin> {
        self.levels
            .iter()
            .flat_map(|l| &l.basins)
            .filter(|b| b.volume > 0 || b.owed > 0)
            .map(|b| SavedBasin {
                cell: *b.cells.iter().min().expect("a basin has cells"),
                volume: b.volume,
                owed: b.owed,
                wet: b.wet(),
                last_volume: b.last_volume,
                rising: b.rising,
            })
            .collect()
    }

    /// What the water has on the map now, to save.
    pub fn saved_costs(&self, map: &Map) -> SavedCosts {
        SavedCosts { cells: map.water_costs().collect(), stale: self.stale.clone() }
    }

    /// Put back what the water had on the map, once the map has its terrain
    /// and things: the costs before rooms and regions are worked out, and
    /// the cells still to clear once the basins are rebuilt.
    pub fn restore_costs(map: &mut Map, saved: &SavedCosts) {
        let n = map.cells();
        for &(c, cost) in saved.cells.iter().filter(|c| (c.0 as usize) < n) {
            map.set_water(c as usize, cost);
        }
    }

    /// The cells of basins rebuilt since the last pass, from a save.
    pub fn restore_stale(&mut self, map: &Map, saved: &SavedCosts) {
        self.stale = saved.stale.iter().copied().filter(|&c| (c as usize) < map.cells()).collect();
    }

    /// Put saved water back, into basins built from the loaded map.
    pub fn restore(&mut self, map: &Map, defs: &DefDb, saved: &[SavedBasin]) {
        self.update(map, defs, true);
        for s in saved {
            let i = s.cell as usize;
            if i >= map.cells() {
                continue;
            }
            let Some(b) = self.basin_of[i].checked_sub(1) else { continue };
            let k = self.level(map.pos(i).z);
            let b = &mut self.levels[k].basins[b as usize];
            b.volume = s.volume.min(b.capacity());
            b.owed = s.owed;
            b.last_volume = s.last_volume;
            b.rising = s.rising;
            b.front = b.rings.partition_point(|&r| r < s.wet) as u32 + (s.wet > 0) as u32;
            b.front = b.front.min(b.rings.len() as u32);
        }
    }
}
