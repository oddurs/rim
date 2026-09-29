//! The tile grid: terrain, one fixture layer (plants, rocks, buildings,
//! blueprints) and one item layer (stacks). Plus reachability regions.
//!
//! The grid has levels (DESIGN.md §6d): every per-cell array holds each
//! level's plane one after another, the surface first, then the levels
//! below it, then those above. So a cell's index is its level's offset plus
//! `y * w + x`, a neighbour is found on the same level by `IVec::offset`,
//! and code that only knows the surface indexes the first plane.

use crate::defs::DefId;
use crate::path::Goal;
use crate::world::Faction;
use crate::IVec;
use hecs::Entity;

pub struct Map {
    pub w: i32,
    pub h: i32,
    /// Levels below the surface and above it.
    below: i32,
    above: i32,
    /// Cells on one level, kept: every index of a level below or above
    /// multiplies by it.
    plane: usize,
    pub terrain: Vec<DefId>,
    pub terrain_cost: Vec<u16>,
    pub fixture: Vec<Option<Entity>>,
    pub item: Vec<Option<Entity>>,
    /// Built ground, under both. A floor never blocks, bounds or stops a
    /// field; it only changes what a step costs.
    pub floor: Vec<Option<Entity>>,
    /// The floor's move cost, standing in for the terrain's; 0 = no floor.
    floor_cost: Vec<u16>,
    fix_block: Vec<bool>,
    /// A fixture here spans air: a drawbridge (DESIGN.md §6d).
    fix_span: Vec<bool>,
    /// A fixture here holds water back: a sealed door.
    fix_holds: Vec<bool>,
    /// What the water standing here adds to a step's cost, in percent;
    /// `DEEP` is past wading and takes the footing away unless a floor or
    /// a span stands over it (DESIGN.md §6d).
    water_cost: Vec<u16>,
    /// Something to stand on: walkable terrain, a floor, or a fixture that
    /// spans air. What `passable_i` reads, kept so a floor over a pit is
    /// ground without the hot path asking three arrays.
    footing: Vec<bool>,
    fix_cost: Vec<u16>,
    /// Extra move cost in percent from what lies on the ground (snow,
    /// mud), kept by the fields that say so. It never blocks, so it moves
    /// no region.
    extra_cost: Vec<u16>,
    fix_door: Vec<bool>,
    /// How far the fixture here holds the roof up, in cells; 0 for none.
    support: Vec<u8>,
    /// How far each terrain holds a roof up, by terrain id: solid rock
    /// does, as the thing it stands up as (DESIGN.md §6d).
    terrain_span: Vec<u8>,
    /// Roof reach left at each cell plus one; 0 is beyond every support.
    /// Worked out with the rooms.
    cover: Vec<u8>,
    /// Supports changed since cover was last worked out, each with the
    /// span it had before: a support taken away reached that far. `None`
    /// until the first full pass, or after too many to patch.
    support_changed: Option<Vec<(u32, u8)>>,
    /// Who owns the fixture here, as `faction as u8 + 1`; 0 is nobody.
    /// Only doors read it: a door opens for its owner and blocks everyone
    /// else, which is what makes a wall with a door in it still a wall.
    fix_owner: Vec<u8>,
    /// Room id per cell (0 = wall, door or impassable).
    room: Vec<u32>,
    /// Room ids from before the last rebuild, so state can carry over.
    prev_room: Vec<u32>,
    /// Cells whose walls or doors changed since the last `take_changed_cells`.
    changed: Vec<u32>,
    rooms: Vec<Room>,
    /// Boundary cells per room (walls, doors, rock, water), indexed by
    /// room id - 1. Collected during the room flood fill, so it is free.
    room_boundary: Vec<Vec<u32>>,
    /// Which room last listed each cell as boundary, so a wall touching
    /// three cells of one room is listed once.
    bound_seen: Vec<u32>,
    rooms_dirty: bool,
    /// How many times rooms have been rebuilt (they only are when walls change).
    pub room_rebuilds: u64,
    /// How many level-by-faction region layers have been rebuilt.
    pub region_rebuilds: u64,
    /// Connected-component id per cell (0 = impassable), one layer per
    /// faction, indexed by `Faction as usize`. They differ only where an
    /// owned door stands: what a raider can walk to is not what the owner
    /// can. Lets us reject unreachable targets in O(1) before running A*.
    regions: [Vec<u32>; Faction::ALL.len()],
    /// One bit per level whose regions need rebuilding (by plane, in the
    /// order the arrays hold them).
    regions_dirty: u64,
    /// The ways between levels (DESIGN.md §6d): stairs, ladders. Sorted by
    /// the top cell's index.
    portals: Vec<Portal>,
    /// Per cell: the index of the other end of a portal standing here, plus
    /// one; 0 is none. What A* steps through.
    link: Vec<u32>,
    /// Per terrain: is it air, a cell with no floor (a pit, a shaft).
    terrain_air: Vec<bool>,
    /// Per terrain: is it water that never runs out (DESIGN.md §6d).
    terrain_pours: Vec<bool>,
    /// Air cells per level, by plane, sorted: what light and water cross.
    air: Vec<Vec<u32>>,
    /// Region ids joined at portals, one table per faction: a region
    /// that no portal touches is its own.
    reach: [std::collections::BTreeMap<u32, u32>; Faction::ALL.len()],
    /// The same for creatures that climb a pit's side (a `[[movement]]`
    /// with `drop`), joined at the pits as well; kept only when some
    /// creature loaded climbs.
    climb_reach: [std::collections::BTreeMap<u32, u32>; Faction::ALL.len()],
    climbers: bool,
    reach_dirty: bool,
    /// Bumped whenever passability changes; renderers can use it to cache.
    pub revision: u64,
    /// The same, per level, by plane: what changed where, so work that only
    /// reads the surface (wind) isn't redone for a dig underground.
    level_rev: Vec<u64>,
    /// One bit per cell, set once a pawn has stood beside it (DESIGN.md
    /// §6d): rock shows what it's made of once seen, and prospecting reads
    /// it. The surface and what is over it start seen.
    seen: Vec<u64>,
    /// The same, for what water can stand in: terrain, and whether a
    /// fixture blocks. Placing furniture moves `level_rev` and not this, so
    /// basins aren't rebuilt for it.
    water_rev: Vec<u64>,
    /// Per level, bumped when a cell becomes or stops being air or water
    /// that pours: all the level below reads of it for its basins.
    ground_rev: Vec<u64>,
    /// Chunks across (see `CHUNK`).
    chunks_w: i32,
    /// Per chunk: bumped when a cell's terrain changes.
    terrain_rev: Vec<u64>,
    /// How far each cell is from each terrain tag terms read.
    near: crate::near::Near,
    /// Per chunk: bumped when anything drawn in a cell changes (what is on
    /// it, or how it looks), and at a chunk's border when a neighbour's does,
    /// since joined walls look at their neighbours. Not a plan's progress:
    /// that moves every tick of work, so renderers draw plans each frame.
    things_rev: Vec<u64>,
    /// Per chunk: bumped when a fixture is placed or removed. Not items,
    /// designations or looks, so what cares only about what stands in a
    /// cell (the renderer's shadows) isn't woken by hauling.
    fixture_rev: Vec<u64>,
}

/// Side of a chunk, in cells: the one unit caches and incremental updates
/// key on (DESIGN.md §6a). Consumers remember the revision they last saw;
/// the sim never reads them, so they are not world state.
pub const CHUNK: i32 = 32;

/// A way between two levels: the top cell and the one below it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Portal {
    pub top: IVec,
    pub bottom: IVec,
    /// Move cost in percent, as a terrain's: 250 is stairs.
    pub cost: u16,
    /// Who may use it, as `faction as u8 + 1`; 0 is anyone. An owned
    /// portal is a wall to everyone else, as an owned door is.
    pub owner: u8,
}

impl Portal {
    pub fn open_to(&self, who: Faction) -> bool {
        self.owner == 0 || self.owner == who as u8 + 1
    }
}

/// A connected area bounded by walls, doors, rock, water or the map edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Room {
    pub id: u32,
    /// The level it is on: a room never spans two.
    pub level: i32,
    pub cells: u32,
    pub touches_edge: bool,
    /// Cells beyond the reach of every support: open to the sky.
    pub uncovered: u32,
}

impl Room {
    /// Shelter: cut off from the map edge, and roofed everywhere, which is
    /// to say within span of a wall, a pillar or rock (DESIGN.md §6c). A
    /// valley ringed by cliffs is not a house.
    pub fn enclosed(&self) -> bool {
        !self.touches_edge && self.uncovered == 0
    }
}

/// Water past wading: see `Map::set_water`.
pub const DEEP: u16 = u16::MAX;

pub const NEIGHBORS8: [(i32, i32); 8] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)];

impl Map {
    /// A map that is only its surface.
    pub fn new(w: i32, h: i32) -> Self {
        Self::with_levels(w, h, 0, 0)
    }

    /// A map with `below` levels under the surface and `above` over it. The
    /// levels besides the surface start impassable, for map generation to
    /// fill in.
    pub fn with_levels(w: i32, h: i32, below: i32, above: i32) -> Self {
        assert!(below >= 0 && above >= 0 && below + above < 64, "at most 64 levels");
        // Each level's region ids fit under its plane's number times 1 << 20.
        assert!(w * h <= 1 << 20, "a level is at most 1,048,576 cells");
        let plane = (w * h) as usize;
        let n = plane * (below + above + 1) as usize;
        let mut terrain_cost = vec![0; n];
        terrain_cost[..plane].fill(100);
        let chunks = Self::chunk_count(w, h) * (below + above + 1) as usize;
        Map {
            w,
            h,
            below,
            above,
            plane,
            terrain: vec![0; n],
            terrain_cost,
            fixture: vec![None; n],
            item: vec![None; n],
            floor: vec![None; n],
            floor_cost: vec![0; n],
            fix_block: vec![false; n],
            fix_span: vec![false; n],
            fix_holds: vec![false; n],
            water_cost: vec![0; n],
            footing: (0..n).map(|i| i < plane).collect(),
            fix_cost: vec![0; n],
            extra_cost: vec![0; n],
            fix_door: vec![false; n],
            support: vec![0; n],
            terrain_span: Vec::new(),
            cover: vec![0; n],
            support_changed: None,
            fix_owner: vec![0; n],
            room: vec![0; n],
            prev_room: vec![0; n],
            changed: Vec::new(),
            rooms: Vec::new(),
            room_boundary: Vec::new(),
            bound_seen: vec![0; n],
            rooms_dirty: true,
            room_rebuilds: 0,
            region_rebuilds: 0,
            regions: std::array::from_fn(|_| vec![0; n]),
            regions_dirty: (1u64 << (below + above + 1)) - 1,
            portals: Vec::new(),
            link: vec![0; n],
            terrain_air: Vec::new(),
            terrain_pours: Vec::new(),
            air: vec![Vec::new(); (below + above + 1) as usize],
            reach: std::array::from_fn(|_| std::collections::BTreeMap::new()),
            climb_reach: std::array::from_fn(|_| std::collections::BTreeMap::new()),
            climbers: false,
            reach_dirty: true,
            level_rev: vec![0; (below + above + 1) as usize],
            water_rev: vec![0; (below + above + 1) as usize],
            ground_rev: vec![0; (below + above + 1) as usize],
            revision: 0,
            seen: {
                let mut bits = vec![0u64; n.div_ceil(64)];
                let unseen = plane..plane * (below as usize + 1);
                for i in (0..n).filter(|i| !unseen.contains(i)) {
                    bits[i / 64] |= 1 << (i % 64);
                }
                bits
            },
            chunks_w: (w + CHUNK - 1) / CHUNK,
            terrain_rev: vec![0; chunks],
            things_rev: vec![0; chunks],
            fixture_rev: vec![0; chunks],
            near: Default::default(),
        }
    }

    /// The levels there are, lowest first.
    pub fn levels(&self) -> std::ops::RangeInclusive<i32> {
        -self.below..=self.above
    }

    /// Every cell on every level: the length of each per-cell array.
    pub fn cells(&self) -> usize {
        self.fixture.len()
    }

    /// Cells on one level.
    pub fn plane(&self) -> usize {
        self.plane
    }

    /// Where level `z` sits among the planes: the surface first, then down,
    /// then up.
    #[inline]
    fn slot(&self, z: i32) -> usize {
        (if z <= 0 { -z } else { self.below + z }) as usize
    }

    #[inline]
    fn level_of_slot(&self, slot: usize) -> i32 {
        let s = slot as i32;
        if s <= self.below {
            -s
        } else {
            s - self.below
        }
    }

    fn chunk_count(w: i32, h: i32) -> usize {
        (((w + CHUNK - 1) / CHUNK) * ((h + CHUNK - 1) / CHUNK)) as usize
    }

    /// Chunks across and down, on one level. The surface's chunks come
    /// first, so chunk `c` below `chunks().0 * chunks().1` is on the surface.
    pub fn chunks(&self) -> (i32, i32) {
        (self.chunks_w, (self.h + CHUNK - 1) / CHUNK)
    }

    /// The top-left cell of chunk `c`, on its level.
    pub fn chunk_origin(&self, c: usize) -> IVec {
        let per = Self::chunk_count(self.w, self.h);
        let (slot, c) = ((c / per), (c % per) as i32);
        IVec::at((c % self.chunks_w) * CHUNK, (c / self.chunks_w) * CHUNK, self.level_of_slot(slot))
    }

    /// The chunks of level `z`, as indices.
    pub fn level_chunks(&self, z: i32) -> std::ops::Range<usize> {
        let per = Self::chunk_count(self.w, self.h);
        let start = self.slot(z) * per;
        start..start + per
    }

    /// What stands in cell `i`, in the order it is drawn: floor, item, fixture.
    pub fn layers_at(&self, i: usize) -> [Option<Entity>; 3] {
        [self.floor[i], self.item[i], self.fixture[i]]
    }

    pub fn chunk_of(&self, p: IVec) -> usize {
        self.slot(p.z) * Self::chunk_count(self.w, self.h) + ((p.y / CHUNK) * self.chunks_w + p.x / CHUNK) as usize
    }

    pub fn terrain_rev(&self, chunk: usize) -> u64 {
        self.terrain_rev[chunk]
    }

    pub fn fixture_rev(&self, chunk: usize) -> u64 {
        self.fixture_rev[chunk]
    }

    pub fn things_rev(&self, chunk: usize) -> u64 {
        self.things_rev[chunk]
    }

    /// Something drawn at `p` changed. Call it for changes the map can't
    /// see: a stack's count, a designation, a plant picked clean.
    pub fn touch(&mut self, p: IVec) {
        if !self.inb(p) {
            return;
        }
        let c = self.chunk_of(p);
        self.things_rev[c] += 1;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let q = p.offset(dx, dy);
            if self.inb(q) && self.chunk_of(q) != c {
                let d = self.chunk_of(q);
                self.things_rev[d] += 1;
            }
        }
    }

    /// Has a pawn stood beside cell `i`?
    #[inline]
    pub fn seen(&self, i: usize) -> bool {
        self.seen[i / 64] & (1 << (i % 64)) != 0
    }

    /// A pawn stands at `p`: it and the cells around it are seen. A cell
    /// seen for the first time is touched, since rock draws differently.
    pub fn see_around(&mut self, p: IVec) {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let q = p.offset(dx, dy);
                if !self.inb(q) {
                    continue;
                }
                let i = self.idx(q);
                if !self.seen(i) {
                    self.seen[i / 64] |= 1 << (i % 64);
                    self.touch(q);
                }
            }
        }
    }

    /// The seen bits, for a save.
    pub fn seen_bits(&self) -> &[u64] {
        &self.seen
    }

    /// Seen bits from a save; ignored if they don't fit this map.
    pub fn set_seen_bits(&mut self, bits: Vec<u64>) {
        if bits.len() == self.seen.len() {
            self.seen = bits;
        }
    }

    /// Put `e` in the item slot at `p`.
    pub fn set_item(&mut self, p: IVec, e: Option<Entity>) {
        let i = self.idx(p);
        self.item[i] = e;
        self.touch(p);
    }

    #[inline]
    pub fn inb(&self, p: IVec) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < self.w && p.y < self.h && p.z >= -self.below && p.z <= self.above
    }
    #[inline]
    pub fn idx(&self, p: IVec) -> usize {
        let flat = (p.y * self.w + p.x) as usize;
        // The surface is the first plane: no offset, and the common case.
        if p.z == 0 {
            flat
        } else {
            self.slot(p.z) * self.plane + flat
        }
    }
    #[inline]
    pub fn pos(&self, i: usize) -> IVec {
        if i < self.plane {
            return IVec::new(i as i32 % self.w, i as i32 / self.w);
        }
        let (slot, r) = (i / self.plane, (i % self.plane) as i32);
        IVec::at(r % self.w, r / self.w, self.level_of_slot(slot))
    }
    #[inline]
    pub fn passable_i(&self, i: usize) -> bool {
        self.footing[i] && !self.fix_block[i]
    }
    #[inline]
    pub fn passable(&self, p: IVec) -> bool {
        self.inb(p) && self.passable_i(self.idx(p))
    }
    /// Movement cost in percent (100 = open ground).
    #[inline]
    pub fn cost(&self, p: IVec) -> u32 {
        let i = self.idx(p);
        let ground = match (self.floor_cost[i], self.terrain_cost[i]) {
            (0, 0) => 100,
            (0, t) => t,
            (f, _) => f,
        };
        let water = match self.water_cost[i] {
            DEEP => 0,
            w => w,
        };
        ground as u32 + self.fix_cost[i] as u32 + self.extra_cost[i] as u32 + water as u32
    }

    /// Change the extra move cost at cell `i` by `delta` percent.
    pub fn add_extra_cost(&mut self, i: usize, delta: i32) {
        let c = &mut self.extra_cost[i];
        *c = (*c as i32 + delta).clamp(0, u16::MAX as i32) as u16;
    }

    /// The extra move cost at cell `i`, in percent.
    pub fn extra_cost(&self, i: usize) -> u16 {
        self.extra_cost[i]
    }

    pub fn set_terrain(&mut self, p: IVec, def: DefId, cost: u32) {
        let i = self.idx(p);
        let was = self.span_at(i);
        self.near.changed(i, self.terrain[i], def);
        let was_air = self.is_air(i);
        let was_terrain = self.terrain[i];
        self.terrain[i] = def;
        if self.is_air(i) != was_air {
            let list = &mut self.air[i / self.plane];
            match list.binary_search(&(i as u32)) {
                Ok(_) if !was_air => {}
                Ok(k) => {
                    list.remove(k);
                }
                Err(k) => list.insert(k, i as u32),
            }
        }
        if self.span_at(i) != was && !self.under_rock(i) {
            if let Some(c) = &mut self.support_changed {
                c.push((i as u32, was));
            }
        }
        self.terrain_cost[i] = cost.min(u16::MAX as u32) as u16;
        self.footing[i] = self.has_footing(i);
        self.water_rev[i / self.plane] += 1;
        let wet = |m: &Self, t: DefId| {
            let t = t as usize;
            m.terrain_air.get(t).copied().unwrap_or(false) || m.terrain_pours.get(t).copied().unwrap_or(false)
        };
        if wet(self, was_terrain) != wet(self, def) {
            self.ground_rev[i / self.plane] += 1;
        }
        self.dirty_regions(i);
        self.rooms_dirty = true;
        self.changed.push(i as u32);
        self.bump(i);
        let c = self.chunk_of(p);
        self.terrain_rev[c] += 1;
    }

    pub fn set_fixture(&mut self, p: IVec, e: Option<Entity>, blocks: bool, cost: u32, door: bool) {
        let i = self.idx(p);
        self.fixture[i] = e;
        // Only what changes passage matters: a thing standing up in rock,
        // which nothing could walk into anyway, rebuilds nothing. An owned
        // door is a wall to everyone but its owner, so gaining or losing
        // one changes who can reach what.
        let was = (self.passable_i(i), self.fix_door[i]);
        if self.fix_block[i] != blocks {
            self.water_rev[i / self.plane] += 1;
        }
        self.fix_block[i] = blocks;
        self.fix_door[i] = door;
        if e.is_none() {
            self.fix_span[i] = false;
            if self.fix_holds[i] {
                self.fix_holds[i] = false;
                self.water_rev[i / self.plane] += 1;
            }
            self.footing[i] = self.has_footing(i);
        }
        if (self.passable_i(i), self.fix_door[i]) != was {
            self.dirty_regions(i);
            self.rooms_dirty = true;
            self.changed.push(i as u32);
        }
        self.fix_cost[i] = cost.min(u16::MAX as u32) as u16;
        self.bump(i);
        let c = self.chunk_of(p);
        self.fixture_rev[c] += 1;
        self.touch(p);
    }

    pub fn fixture_at(&self, p: IVec) -> Option<Entity> {
        if self.inb(p) {
            self.fixture[self.idx(p)]
        } else {
            None
        }
    }
    pub fn item_at(&self, p: IVec) -> Option<Entity> {
        if self.inb(p) {
            self.item[self.idx(p)]
        } else {
            None
        }
    }
    pub fn floor_at(&self, p: IVec) -> Option<Entity> {
        if self.inb(p) {
            self.floor[self.idx(p)]
        } else {
            None
        }
    }

    /// Lay or lift a floor. `cost` 0 is a floor that changes nothing yet
    /// (a blueprint). Over air a floor is the only footing, so laying or
    /// lifting one there changes who can reach what.
    pub fn set_floor(&mut self, p: IVec, e: Option<Entity>, cost: u32) {
        let i = self.idx(p);
        self.floor[i] = e;
        self.floor_cost[i] = if e.is_some() { cost.min(u16::MAX as u32) as u16 } else { 0 };
        self.refoot(i);
        self.bump(i);
        self.touch(p);
    }

    /// The fixture at `p` spans air (a drawbridge), or no longer does.
    /// Removing the fixture clears it.
    pub fn set_span(&mut self, p: IVec, spans: bool) {
        let i = self.idx(p);
        self.fix_span[i] = spans;
        self.refoot(i);
        self.bump(i);
    }

    /// The fixture at `p` holds water back (a sealed door), or doesn't.
    /// Removing the fixture clears it.
    pub fn set_holds_water(&mut self, p: IVec, holds: bool) {
        let i = self.idx(p);
        if self.fix_holds[i] != holds {
            self.fix_holds[i] = holds;
            self.water_rev[i / self.plane] += 1;
        }
    }

    /// Water here adds `cost` percent to a step, or with `DEEP` takes the
    /// footing away. Returns whether that changed who can walk it.
    pub fn set_water(&mut self, i: usize, cost: u16) -> bool {
        if self.water_cost[i] == cost {
            return false;
        }
        let was = self.footing[i];
        self.water_cost[i] = cost;
        self.refoot(i);
        self.bump(i);
        self.footing[i] != was
    }

    /// Could `i` be walked if the water in it were gone? What someone
    /// scrambling out of water may step through.
    pub fn passable_wet(&self, i: usize) -> bool {
        (self.terrain_cost[i] > 0 || self.floor_cost[i] > 0 || self.fix_span[i]) && !self.fix_block[i]
    }

    fn has_footing(&self, i: usize) -> bool {
        (self.terrain_cost[i] > 0 && self.water_cost[i] != DEEP) || self.floor_cost[i] > 0 || self.fix_span[i]
    }

    /// Work out cell `i`'s footing again, rebuilding what reads it when it
    /// changed.
    fn refoot(&mut self, i: usize) {
        let now = self.has_footing(i);
        if now != self.footing[i] {
            self.footing[i] = now;
            self.dirty_regions(i);
            self.rooms_dirty = true;
            self.changed.push(i as u32);
        }
    }

    /// Cell `i`'s level needs its regions rebuilt.
    fn dirty_regions(&mut self, i: usize) {
        self.regions_dirty |= 1 << (i / self.plane());
    }

    /// Rebuild the regions of the levels whose passability changed, and
    /// only those: a level nobody touched keeps its ids. A level's ids start
    /// at its plane's number times `1 << 20`, so they never collide.
    pub fn ensure_regions(&mut self) {
        if self.regions_dirty == 0 {
            if self.reach_dirty {
                self.join_reach();
            }
            return;
        }
        self.reach_dirty = true;
        let dirty = std::mem::take(&mut self.regions_dirty);
        let plane = self.plane();
        let mut stack = Vec::new();
        for who in Faction::ALL {
            let open = |m: &Self, i: usize| m.passable_i(i) && !m.locked_against(i, who);
            let mut region = std::mem::take(&mut self.regions[who as usize]);
            for slot in (0..64).filter(|s| dirty & (1 << s) != 0) {
                let cells = slot * plane..(slot + 1) * plane;
                region[cells.clone()].iter_mut().for_each(|r| *r = 0);
                self.region_rebuilds += 1;
                self.flood_regions(&mut region, cells, (slot as u32) << 20, &open, &mut stack);
            }
            self.regions[who as usize] = region;
        }
        self.join_reach();
    }

    /// Join regions at the portals that link them, per faction: one pass
    /// over the portals, in order, so the result is the same every time.
    fn join_reach(&mut self) {
        self.reach_dirty = false;
        type Roots = std::collections::BTreeMap<u32, u32>;
        fn find(roots: &mut Roots, r: u32) -> u32 {
            let mut x = r;
            while let Some(&up) = roots.get(&x).filter(|&&up| up != x) {
                x = up;
            }
            roots.insert(r, x);
            x
        }
        fn join(roots: &mut Roots, a: u32, b: u32) {
            if a == 0 || b == 0 {
                return;
            }
            let (ra, rb) = (find(roots, a), find(roots, b));
            // The smaller id is the root, so the order joins came in
            // doesn't matter.
            let (lo, hi) = (ra.min(rb), ra.max(rb));
            roots.insert(hi, lo);
            roots.insert(lo, lo);
        }
        fn settle(roots: &mut Roots) {
            let keys: Vec<u32> = roots.keys().copied().collect();
            for k in keys {
                find(roots, k);
            }
        }
        for who in Faction::ALL {
            let layer = &self.regions[who as usize];
            let mut roots = Roots::new();
            for p in self.portals.iter().filter(|p| p.open_to(who)) {
                join(&mut roots, layer[self.idx(p.top)], layer[self.idx(p.bottom)]);
            }
            if self.climbers {
                // A pit's side: the ground beside it at the top joins the
                // floor it drops to.
                let mut climb = roots.clone();
                for air in &self.air {
                    for &a in air {
                        let a = a as usize;
                        let top = self.pos(a);
                        if self.passable_i(a) || top.z <= -self.below {
                            continue;
                        }
                        let b = self.idx(IVec::at(top.x, top.y, top.z - 1));
                        if !self.passable_for(self.pos(b), who) {
                            continue;
                        }
                        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                            let n = top.offset(dx, dy);
                            if self.passable_for(n, who) {
                                join(&mut climb, layer[self.idx(n)], layer[b]);
                            }
                        }
                    }
                }
                settle(&mut climb);
                self.climb_reach[who as usize] = climb;
            }
            settle(&mut roots);
            self.reach[who as usize] = roots;
        }
    }

    /// Which joined area region `r` belongs to, for `who`, climbing or not.
    fn reach_of(&self, r: u32, who: Faction, climbs: bool) -> u32 {
        let table = if climbs && self.climbers { &self.climb_reach } else { &self.reach };
        table[who as usize].get(&r).copied().unwrap_or(r)
    }

    /// Some creature climbs a pit's side: keep reach for them too.
    pub fn set_climbers(&mut self, on: bool) {
        self.climbers = on;
        self.reach_dirty = true;
    }

    /// The ways a climber goes from cell `i` besides walking (DESIGN.md
    /// §6d): down a pit's side beside it, or up the side of the pit over
    /// it. None where no creature climbs.
    pub fn climbs(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        let p = self.pos(i);
        let side = move |dx: i32, dy: i32| p.offset(dx, dy);
        let down = [(1, 0), (-1, 0), (0, 1), (0, -1)].into_iter().filter_map(move |(dx, dy)| {
            let a = side(dx, dy);
            let ok = self.climbers && self.inb(a) && p.z > -self.below && self.is_air(self.idx(a)) && !self.passable(a);
            ok.then(|| IVec::at(a.x, a.y, a.z - 1)).filter(|b| self.passable(*b)).map(|b| self.idx(b))
        });
        let over = IVec::at(p.x, p.y, p.z + 1);
        let pit = self.climbers && self.inb(over) && self.is_air(self.idx(over)) && !self.passable(over);
        let up = [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .into_iter()
            .filter(move |_| pit)
            .map(move |(dx, dy)| over.offset(dx, dy))
            .filter(|n| self.passable(*n))
            .map(|n| self.idx(n));
        down.chain(up)
    }

    /// Number the connected areas among `cells` (one level), from `base + 1`.
    fn flood_regions(
        &self,
        region: &mut [u32],
        cells: std::ops::Range<usize>,
        base: u32,
        open: &impl Fn(&Self, usize) -> bool,
        stack: &mut Vec<usize>,
    ) {
        let mut next = base + 1;
        for start in cells {
            if region[start] != 0 || !open(self, start) {
                continue;
            }
            region[start] = next;
            stack.push(start);
            while let Some(i) = stack.pop() {
                let p = self.pos(i);
                // 4-connected is exact: diagonal moves need both orthogonals open.
                for (dx, dy) in &NEIGHBORS8[..4] {
                    let q = p.offset(*dx, *dy);
                    if !self.inb(q) {
                        continue;
                    }
                    let j = self.idx(q);
                    if region[j] == 0 && open(self, j) {
                        region[j] = next;
                        stack.push(j);
                    }
                }
            }
            next += 1;
        }
    }

    /// A door that `who` does not own. Passable to its owner, a wall to
    /// everyone else, and the only thing the region layers disagree about.
    #[inline]
    pub fn locked_against(&self, i: usize, who: Faction) -> bool {
        self.fix_door[i] && self.fix_owner[i] != 0 && self.fix_owner[i] != who as u8 + 1
    }

    /// Can `who` walk into `p` without breaking something?
    #[inline]
    pub fn passable_for(&self, p: IVec, who: Faction) -> bool {
        self.inb(p) && self.passable_i(self.idx(p)) && !self.locked_against(self.idx(p), who)
    }

    /// Give the fixture at `p` an owner, or take ownership away.
    pub fn set_owner(&mut self, p: IVec, owner: Option<Faction>) {
        let i = self.idx(p);
        let v = owner.map_or(0, |f| f as u8 + 1);
        if self.fix_owner[i] == v {
            return;
        }
        self.fix_owner[i] = v;
        self.dirty_regions(i);
        self.bump(i);
    }

    pub fn owner_at(&self, p: IVec) -> Option<Faction> {
        let v = self.fix_owner[self.idx(p)];
        (v > 0).then(|| Faction::ALL[v as usize - 1])
    }

    /// Region id as `who` sees it. Call `ensure_regions` first.
    pub fn region_at_for(&self, p: IVec, who: Faction) -> u32 {
        if self.inb(p) {
            self.regions[who as usize][self.idx(p)]
        } else {
            0
        }
    }

    /// Region id ignoring ownership, for callers that only care about the
    /// shape of the land (map generation, plant spread).
    pub fn region_at(&self, p: IVec) -> u32 {
        self.region_at_for(p, Faction::Player)
    }

    /// Cheap reachability test for a colonist. Call `ensure_regions` first.
    pub fn can_reach(&self, from: IVec, goal: Goal) -> bool {
        self.can_reach_for(from, goal, Faction::Player)
    }

    /// Cheap reachability test, from `who`'s side of the doors.
    pub fn can_reach_for(&self, from: IVec, goal: Goal, who: Faction) -> bool {
        self.can_reach_as(from, goal, who, false)
    }

    /// `can_reach_for`, for a creature that climbs a pit's side or not.
    pub fn can_reach_as(&self, from: IVec, goal: Goal, who: Faction, climbs: bool) -> bool {
        // Regions joined at the portals: a cell below is reached by stairs.
        let region_at = |p: IVec| match self.region_at_for(p, who) {
            0 => 0,
            r => self.reach_of(r, who, climbs),
        };
        let rf = region_at(from);
        if rf == 0 {
            return true; // standing somewhere odd (fresh wall): let A* decide
        }
        match goal {
            Goal::Cell(c) => region_at(c) == rf,
            Goal::Touch(_) | Goal::Area { .. } => {
                goal.cells().any(|c| (-1..=1).any(|dy| (-1..=1).any(|dx| region_at(c.offset(dx, dy)) == rf)))
            }
        }
    }

    /// Rebuild rooms if a wall, door or terrain changed since the last call.
    pub fn ensure_rooms(&mut self) {
        if !self.rooms_dirty {
            return;
        }
        self.rooms_dirty = false;
        self.room_rebuilds += 1;
        std::mem::swap(&mut self.room, &mut self.prev_room);
        self.room.iter_mut().for_each(|r| *r = 0);
        self.bound_seen.iter_mut().for_each(|r| *r = 0);
        self.rooms.clear();
        self.room_boundary.clear();
        let mut stack = Vec::new();
        for start in 0..self.room.len() {
            if self.room[start] != 0 || !self.room_cell(start) {
                continue;
            }
            let id = self.rooms.len() as u32 + 1;
            let level = self.pos(start).z;
            let mut room = Room { id, level, cells: 0, touches_edge: false, uncovered: 0 };
            let mut boundary = Vec::new();
            self.room[start] = id;
            stack.push(start);
            while let Some(i) = stack.pop() {
                let p = self.pos(i);
                room.cells += 1;
                if p.x == 0 || p.y == 0 || p.x == self.w - 1 || p.y == self.h - 1 {
                    room.touches_edge = true;
                }
                // The fill is 4-connected; the boundary is everything that
                // touches the room, corners included, so a ring of wall is
                // all of its pieces and not just the ones facing inward.
                for (k, (dx, dy)) in NEIGHBORS8.iter().enumerate() {
                    let q = p.offset(*dx, *dy);
                    if !self.inb(q) {
                        continue;
                    }
                    let j = self.idx(q);
                    if !self.room_cell(j) {
                        if self.bound_seen[j] != id {
                            self.bound_seen[j] = id;
                            boundary.push(j as u32);
                        }
                    } else if k < 4 && self.room[j] == 0 {
                        self.room[j] = id;
                        stack.push(j);
                    }
                }
            }
            self.rooms.push(room);
            self.room_boundary.push(boundary);
        }
        self.spread_cover();
        for (i, &id) in self.room.iter().enumerate() {
            if id > 0 && !self.covered(i) {
                self.rooms[id as usize - 1].uncovered += 1;
            }
        }
    }

    /// How far the roof reaches from every support: a support's reach is
    /// its span, and each step to any of the 8 neighbours costs one. A
    /// changed support can only move cover within the longest span of it,
    /// so a few changes are patched in place; the first pass, and a big
    /// batch (map generation, a load), do the whole map.
    fn spread_cover(&mut self) {
        match self.support_changed.take() {
            Some(changed) if changed.len() <= 32 => {
                let top = self.roofed_cells().map(|i| self.span_at(i)).max().unwrap_or(0).max(1) as i32;
                for (i, was) in changed {
                    // Every cell a support here could have reached, or can:
                    // what it held before may be wider than anything left.
                    self.cover_window(self.pos(i as usize), top.max(was as i32));
                }
            }
            _ => {
                let (w, h) = (self.w, self.h);
                for z in 0..=self.above {
                    self.cover_window(IVec::at(w / 2, h / 2, z), w.max(h));
                }
            }
        }
        self.support_changed = Some(Vec::new());
    }

    /// Work cover out again for the cells within `r` of `p`. Cover is the
    /// best of each neighbour's less one, so the ring just outside the
    /// window, whose values nothing inside can have changed, is all the
    /// window needs from beyond it: those cells seed the fill with what
    /// they hold, beside the supports inside. The window is on `p`'s level:
    /// a roof holds up the level it stands on.
    fn cover_window(&mut self, p: IVec, r: i32) {
        let (x0, y0) = ((p.x - r).max(0), (p.y - r).max(0));
        let (x1, y1) = ((p.x + r).min(self.w - 1), (p.y + r).min(self.h - 1));
        let inside = |q: IVec| q.x >= x0 && q.y >= y0 && q.x <= x1 && q.y <= y1;
        let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); crate::defs::MAX_SPAN as usize + 1];
        for y in y0..=y1 {
            for x in x0..=x1 {
                let i = self.idx(IVec::at(x, y, p.z));
                let s = self.span_at(i);
                self.cover[i] = if s > 0 { s + 1 } else { 0 };
                if s > 0 {
                    buckets[s as usize].push(i as u32);
                }
            }
        }
        for y in y0 - 1..=y1 + 1 {
            for x in x0 - 1..=x1 + 1 {
                let q = IVec::at(x, y, p.z);
                if inside(q) || !self.inb(q) {
                    continue;
                }
                let c = self.cover[self.idx(q)];
                if c > 1 {
                    buckets[c as usize - 1].push(self.idx(q) as u32);
                }
            }
        }
        for reach in (1..=crate::defs::MAX_SPAN).rev() {
            for i in std::mem::take(&mut buckets[reach as usize]) {
                if self.cover[i as usize] != reach + 1 {
                    continue; // reached again from a nearer support
                }
                let q = self.pos(i as usize);
                for (dx, dy) in NEIGHBORS8 {
                    let n = q.offset(dx, dy);
                    if !inside(n) {
                        continue;
                    }
                    let j = self.idx(n);
                    if self.cover[j] < reach {
                        self.cover[j] = reach;
                        if reach > 1 {
                            buckets[reach as usize - 1].push(j as u32);
                        }
                    }
                }
            }
        }
    }

    /// How far each terrain holds a roof up, by terrain id. Set once, from
    /// the defs, when the world is made.
    /// Which terrains are air, by terrain id. Set once, from the defs, when
    /// the world is made.
    /// Which terrains are water that never runs out.
    pub fn set_terrain_pours(&mut self, pours: Vec<bool>) {
        self.terrain_pours = pours;
    }

    pub fn set_terrain_air(&mut self, air: Vec<bool>) {
        self.terrain_air = air;
        for list in &mut self.air {
            list.clear();
        }
        for i in 0..self.terrain.len() {
            if self.is_air(i) {
                self.air[i / self.plane].push(i as u32);
            }
        }
    }

    /// A cell with no floor: nothing walks it, and things fall through.
    #[inline]
    pub fn is_air(&self, i: usize) -> bool {
        self.terrain_air.get(self.terrain[i] as usize).copied().unwrap_or(false)
    }

    /// The air cells of level `z`, by index: what light and water cross.
    pub fn air_cells(&self, z: i32) -> &[u32] {
        if !self.levels().contains(&z) {
            return &[];
        }
        &self.air[self.slot(z)]
    }

    /// Something at cell `i` changed what it blocks or costs.
    fn bump(&mut self, i: usize) {
        self.revision += 1;
        self.level_rev[i / self.plane] += 1;
    }

    /// What water can stand in on level `z` changed when this moved.
    pub fn water_revision(&self, z: i32) -> u64 {
        if !self.levels().contains(&z) {
            return 0;
        }
        self.water_rev[self.slot(z)]
    }

    /// A cell of level `z` became or stopped being air or pouring water
    /// when this moved.
    pub fn ground_revision(&self, z: i32) -> u64 {
        if !self.levels().contains(&z) {
            return 0;
        }
        self.ground_rev[self.slot(z)]
    }

    /// Does `fixture` stand in the way of water at cell `i`?
    #[inline]
    pub fn blocks_water(&self, i: usize) -> bool {
        self.fix_block[i] || self.fix_holds[i]
    }

    /// `revision`, for level `z` only.
    pub fn level_revision(&self, z: i32) -> u64 {
        if !self.levels().contains(&z) {
            return 0;
        }
        self.level_rev[self.slot(z)]
    }

    /// Every way between levels, by top cell.
    pub fn portals(&self) -> &[Portal] {
        &self.portals
    }

    /// The portal whose top or bottom is cell `i`.
    pub fn portal_at(&self, i: usize) -> Option<&Portal> {
        let other = self.link[i].checked_sub(1)? as usize;
        let top = if self.pos(i).z > self.pos(other).z { i } else { other };
        let k = self.portals.binary_search_by_key(&top, |p| self.idx(p.top)).ok()?;
        Some(&self.portals[k])
    }

    /// The other end of a portal at cell `i`, if `who` may use it.
    #[inline]
    pub fn through(&self, i: usize, who: Faction) -> Option<(usize, u16)> {
        let other = self.link[i].checked_sub(1)? as usize;
        let p = self.portal_at(i)?;
        p.open_to(who).then_some((other, p.cost))
    }

    /// Join `top` to the cell below it. Both must be on the map.
    pub fn add_portal(&mut self, top: IVec, cost: u16, owner: Option<Faction>) {
        let bottom = IVec::at(top.x, top.y, top.z - 1);
        let (t, b) = (self.idx(top), self.idx(bottom));
        let portal = Portal { top, bottom, cost, owner: owner.map_or(0, |f| f as u8 + 1) };
        match self.portals.binary_search_by_key(&t, |p| self.idx(p.top)) {
            Ok(k) => self.portals[k] = portal,
            Err(k) => self.portals.insert(k, portal),
        }
        self.link[t] = b as u32 + 1;
        self.link[b] = t as u32 + 1;
        self.reach_dirty = true;
        self.bump(t);
        self.bump(b);
        // Water goes down stairs as it does through air.
        self.water_rev[t / self.plane] += 1;
        self.water_rev[b / self.plane] += 1;
    }

    /// Take away the portal whose top is `top`.
    pub fn remove_portal(&mut self, top: IVec) {
        let t = self.idx(top);
        let Ok(k) = self.portals.binary_search_by_key(&t, |p| self.idx(p.top)) else { return };
        let b = self.idx(self.portals[k].bottom);
        self.portals.remove(k);
        self.link[t] = 0;
        self.link[b] = 0;
        self.reach_dirty = true;
        self.bump(t);
        self.bump(b);
        // Water goes down stairs as it does through air.
        self.water_rev[t / self.plane] += 1;
        self.water_rev[b / self.plane] += 1;
    }

    pub fn set_terrain_spans(&mut self, spans: Vec<u8>) {
        self.terrain_span = spans;
        self.support_changed = None;
        self.rooms_dirty = true;
    }

    /// Which terrains have each tag, by tag then terrain id: an empty list
    /// for a tag nothing measures the distance to. Set once, from the defs,
    /// when the world is made.
    pub fn set_near_tags(&mut self, tagged: Vec<Vec<bool>>) {
        self.near = crate::near::Near::new(tagged, self.terrain.len());
    }

    /// Bring the distances to tagged terrain up to date. Cheap when nothing
    /// changed.
    pub fn ensure_near(&mut self) {
        self.near.ensure(self.w, self.h, &self.terrain);
    }

    /// Cells from `i` to the nearest terrain with tag `tag`, on its level, up
    /// to `NEAR_CAP`. Call `ensure_near` first.
    pub fn near(&self, tag: usize, i: usize) -> u8 {
        self.near.at(tag, i)
    }

    /// Cells searched patching distances after terrain changes, for tests.
    pub fn near_patched(&self) -> u64 {
        self.near.patched
    }

    /// The roof a cell holds up: its fixture's or its solid terrain's.
    fn span_at(&self, i: usize) -> u8 {
        let t = self.terrain_span.get(self.terrain[i] as usize).copied().unwrap_or(0);
        self.support[i].max(t)
    }

    /// A support of `span` cells stands at `p` (0: none). Rooms rebuild
    /// when one changes.
    pub fn set_support(&mut self, p: IVec, span: u8) {
        let i = self.idx(p);
        if self.support[i] != span {
            let was = self.span_at(i);
            self.support[i] = span;
            self.rooms_dirty = true;
            let rock = self.under_rock(i);
            if let Some(c) = self.support_changed.as_mut().filter(|_| !rock) {
                c.push((i as u32, was));
            }
        }
    }

    /// Within reach of a support on this level. Call `ensure_rooms` first.
    pub fn covered(&self, i: usize) -> bool {
        self.under_rock(i) || self.cover[i] > 0
    }

    /// Below the surface: rock is the roof over everything there, so cover
    /// is never worked out (DESIGN.md §6d).
    #[inline]
    fn under_rock(&self, i: usize) -> bool {
        let slot = i / self.plane;
        slot >= 1 && slot <= self.below as usize
    }

    /// The cells whose cover is worked out: the surface and the levels
    /// above it.
    fn roofed_cells(&self) -> impl Iterator<Item = usize> + '_ {
        let (plane, below) = (self.plane, self.below as usize);
        (0..plane).chain((below + 1) * plane..self.support.len())
    }

    /// Roofed: within reach of a support. With levels (DESIGN.md §6d) a
    /// cell is also roofed when the cell above it is solid or floored;
    /// until then a level has nothing above it. Call `ensure_rooms` first.
    pub fn roofed(&self, i: usize) -> bool {
        self.covered(i)
    }

    /// The cells enclosing room `id`: walls, doors, rock, water.
    pub fn room_boundary(&self, id: u32) -> &[u32] {
        self.room_boundary.get(id as usize - 1).map_or(&[], |v| v.as_slice())
    }

    /// Heat, light and the like stop at walls, doors and impassable ground.
    pub fn blocks_fields(&self, i: usize) -> bool {
        !self.passable_i(i) || self.fix_door[i]
    }

    /// Cells whose walls, doors or terrain changed since the last call.
    pub fn take_changed_cells(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.changed)
    }

    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    pub fn room_by_id(&self, id: u32) -> Room {
        self.rooms[id as usize - 1]
    }

    /// Rooms need rebuilding: something changed since the last `ensure_rooms`.
    pub fn rooms_dirty(&self) -> bool {
        self.rooms_dirty
    }

    /// The room ids the next carry-over of room values will read: the
    /// current grid if the fields have seen the last rebuild (or one is
    /// still to come), else the grid from before it.
    pub fn carry_from(&self, seen_rebuilds: u64) -> &[u32] {
        if self.rooms_dirty || self.room_rebuilds == seen_rebuilds {
            &self.room
        } else {
            &self.prev_room
        }
    }

    /// After a load: the rooms as the saved room values knew them, so the
    /// fields carry values over from them as the live game would have.
    pub fn set_prev_rooms(&mut self, ids: Vec<u32>) {
        self.prev_room = ids;
    }

    /// (current, previous) room id of a cell; previous is from before the last rebuild.
    pub fn room_ids(&self, i: usize) -> (u32, u32) {
        (self.room[i], self.prev_room[i])
    }

    /// Open floor that belongs to a room: passable and not a doorway.
    pub fn room_cell(&self, i: usize) -> bool {
        self.passable_i(i) && !self.fix_door[i]
    }

    /// The room at `p`. Call `ensure_rooms` first. Walls and doors have none.
    pub fn room_at(&self, p: IVec) -> Option<Room> {
        if !self.inb(p) {
            return None;
        }
        let id = self.room[self.idx(p)];
        (id > 0).then(|| self.rooms[id as usize - 1])
    }

    /// Sheltered: inside an enclosed room. Call `ensure_rooms` first.
    pub fn indoors(&self, p: IVec) -> bool {
        self.room_at(p).is_some_and(|r| r.enclosed())
    }

    /// Size of the region containing `p` (used by map gen to pick a start).
    pub fn region_size(&self, p: IVec) -> usize {
        let r = self.region_at(p);
        if r == 0 {
            return 0;
        }
        self.regions[Faction::Player as usize].iter().filter(|&&x| x == r).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    /// Cover by definition: the best of every support's span less its
    /// Chebyshev distance, plus one; 0 beyond them all.
    fn brute(m: &Map) -> Vec<u8> {
        let mut out = vec![0u8; m.support.len()];
        for (i, o) in out.iter_mut().enumerate() {
            let p = m.pos(i);
            for (j, &s) in m.support.iter().enumerate() {
                let d = p.chebyshev(m.pos(j));
                if s > 0 && d <= s as i32 {
                    *o = (*o).max(s - d as u8 + 1);
                }
            }
        }
        out
    }

    /// Patching cover around a few changed supports gives exactly what
    /// working it out from scratch does, through adds, removals and spans
    /// that change under a support.
    #[test]
    fn patched_cover_matches_the_whole_map() {
        let mut m = Map::new(40, 30);
        let mut rng = Rng::new(7);
        for round in 0..60 {
            let n = if round == 0 { 50 } else { 1 + rng.below(5) };
            for _ in 0..n {
                let p = IVec::new(rng.below(40) as i32, rng.below(30) as i32);
                let span = if rng.below(3) == 0 { 0 } else { 1 + rng.below(6) as u8 };
                m.set_support(p, span);
            }
            m.ensure_rooms();
            assert_eq!(m.cover, brute(&m), "round {round}");
        }
    }

    /// Taking away, or narrowing, the widest support: what it reached is
    /// wider than any span left standing, and all of it is worked out again.
    #[test]
    fn patched_cover_forgets_the_widest_support() {
        for after in [0, 2] {
            let mut m = Map::new(40, 30);
            m.set_support(IVec::new(10, 10), 5);
            m.set_support(IVec::new(30, 20), 3);
            m.ensure_rooms();
            m.set_support(IVec::new(10, 10), after);
            m.ensure_rooms();
            assert_eq!(m.cover, brute(&m), "narrowed to {after}");
        }
    }
}
