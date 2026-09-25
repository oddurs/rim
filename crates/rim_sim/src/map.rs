//! The tile grid: terrain, one fixture layer (plants, rocks, buildings,
//! blueprints) and one item layer (stacks). Plus reachability regions.

use crate::defs::DefId;
use crate::path::Goal;
use crate::world::Faction;
use crate::IVec;
use hecs::Entity;

pub struct Map {
    pub w: i32,
    pub h: i32,
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
    fix_cost: Vec<u16>,
    fix_door: Vec<bool>,
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
    /// Connected-component id per cell (0 = impassable), one layer per
    /// faction, indexed by `Faction as usize`. They differ only where an
    /// owned door stands: what a raider can walk to is not what the owner
    /// can. Lets us reject unreachable targets in O(1) before running A*.
    regions: [Vec<u32>; Faction::ALL.len()],
    regions_dirty: bool,
    /// Bumped whenever passability changes; renderers can use it to cache.
    pub revision: u64,
    /// Chunks across (see `CHUNK`).
    chunks_w: i32,
    /// Per chunk: bumped when a cell's terrain changes.
    terrain_rev: Vec<u64>,
    /// Per chunk: bumped when anything drawn in a cell changes (what is on
    /// it, or how it looks), and at a chunk's border when a neighbour's does,
    /// since joined walls look at their neighbours. Not a plan's progress:
    /// that moves every tick of work, so renderers draw plans each frame.
    things_rev: Vec<u64>,
}

/// Side of a chunk, in cells: the one unit caches and incremental updates
/// key on (DESIGN.md §6a). Consumers remember the revision they last saw;
/// the sim never reads them, so they are not world state.
pub const CHUNK: i32 = 32;

/// Enclosed areas larger than this count as outdoors: a valley ringed by
/// mountains is not a house.
pub const MAX_ROOM_CELLS: u32 = 400;

/// A connected area bounded by walls, doors, rock, water or the map edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Room {
    pub id: u32,
    pub cells: u32,
    pub touches_edge: bool,
}

impl Room {
    /// Shelter: cut off from the map edge, and small enough to be a building.
    pub fn enclosed(&self) -> bool {
        !self.touches_edge && self.cells <= MAX_ROOM_CELLS
    }
}

pub const NEIGHBORS8: [(i32, i32); 8] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)];

impl Map {
    pub fn new(w: i32, h: i32) -> Self {
        let n = (w * h) as usize;
        Map {
            w,
            h,
            terrain: vec![0; n],
            terrain_cost: vec![100; n],
            fixture: vec![None; n],
            item: vec![None; n],
            floor: vec![None; n],
            floor_cost: vec![0; n],
            fix_block: vec![false; n],
            fix_cost: vec![0; n],
            fix_door: vec![false; n],
            fix_owner: vec![0; n],
            room: vec![0; n],
            prev_room: vec![0; n],
            changed: Vec::new(),
            rooms: Vec::new(),
            room_boundary: Vec::new(),
            bound_seen: vec![0; n],
            rooms_dirty: true,
            room_rebuilds: 0,
            regions: std::array::from_fn(|_| vec![0; n]),
            regions_dirty: true,
            revision: 0,
            chunks_w: (w + CHUNK - 1) / CHUNK,
            terrain_rev: vec![0; Self::chunk_count(w, h)],
            things_rev: vec![0; Self::chunk_count(w, h)],
        }
    }

    fn chunk_count(w: i32, h: i32) -> usize {
        (((w + CHUNK - 1) / CHUNK) * ((h + CHUNK - 1) / CHUNK)) as usize
    }

    /// Chunks across and down.
    pub fn chunks(&self) -> (i32, i32) {
        (self.chunks_w, (self.h + CHUNK - 1) / CHUNK)
    }

    pub fn chunk_of(&self, p: IVec) -> usize {
        ((p.y / CHUNK) * self.chunks_w + p.x / CHUNK) as usize
    }

    pub fn terrain_rev(&self, chunk: usize) -> u64 {
        self.terrain_rev[chunk]
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

    /// Put `e` in the item slot at `p`.
    pub fn set_item(&mut self, p: IVec, e: Option<Entity>) {
        let i = self.idx(p);
        self.item[i] = e;
        self.touch(p);
    }

    #[inline]
    pub fn inb(&self, p: IVec) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < self.w && p.y < self.h
    }
    #[inline]
    pub fn idx(&self, p: IVec) -> usize {
        (p.y * self.w + p.x) as usize
    }
    #[inline]
    pub fn pos(&self, i: usize) -> IVec {
        IVec::new(i as i32 % self.w, i as i32 / self.w)
    }
    #[inline]
    pub fn passable_i(&self, i: usize) -> bool {
        self.terrain_cost[i] > 0 && !self.fix_block[i]
    }
    #[inline]
    pub fn passable(&self, p: IVec) -> bool {
        self.inb(p) && self.passable_i(self.idx(p))
    }
    /// Movement cost in percent (100 = open ground).
    #[inline]
    pub fn cost(&self, p: IVec) -> u32 {
        let i = self.idx(p);
        let ground = if self.floor_cost[i] > 0 { self.floor_cost[i] } else { self.terrain_cost[i] };
        ground as u32 + self.fix_cost[i] as u32
    }

    pub fn set_terrain(&mut self, p: IVec, def: DefId, cost: u32) {
        let i = self.idx(p);
        self.terrain[i] = def;
        self.terrain_cost[i] = cost.min(u16::MAX as u32) as u16;
        self.regions_dirty = true;
        self.rooms_dirty = true;
        self.changed.push(i as u32);
        self.revision += 1;
        let c = self.chunk_of(p);
        self.terrain_rev[c] += 1;
    }

    pub fn set_fixture(&mut self, p: IVec, e: Option<Entity>, blocks: bool, cost: u32, door: bool) {
        let i = self.idx(p);
        self.fixture[i] = e;
        if self.fix_block[i] != blocks {
            self.regions_dirty = true;
            self.rooms_dirty = true;
        }
        if self.fix_door[i] != door {
            self.rooms_dirty = true;
            // An owned door is a wall to everyone but its owner, so gaining
            // or losing one changes who can reach what.
            self.regions_dirty = true;
        }
        if self.fix_block[i] != blocks || self.fix_door[i] != door {
            self.changed.push(i as u32);
        }
        self.fix_block[i] = blocks;
        self.fix_door[i] = door;
        self.fix_cost[i] = cost.min(u16::MAX as u32) as u16;
        self.revision += 1;
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
    /// (a blueprint). Passability never changes, so no region rebuild.
    pub fn set_floor(&mut self, p: IVec, e: Option<Entity>, cost: u32) {
        let i = self.idx(p);
        self.floor[i] = e;
        self.floor_cost[i] = if e.is_some() { cost.min(u16::MAX as u32) as u16 } else { 0 };
        self.revision += 1;
        self.touch(p);
    }

    pub fn ensure_regions(&mut self) {
        if !self.regions_dirty {
            return;
        }
        self.regions_dirty = false;
        let mut stack = Vec::new();
        for who in Faction::ALL {
            let open = |m: &Self, i: usize| m.passable_i(i) && !m.locked_against(i, who);
            let mut region = std::mem::take(&mut self.regions[who as usize]);
            region.iter_mut().for_each(|r| *r = 0);
            let mut next = 1;
            for start in 0..region.len() {
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
            self.regions[who as usize] = region;
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
        self.regions_dirty = true;
        self.revision += 1;
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
        let region_at = |p: IVec| self.region_at_for(p, who);
        let rf = region_at(from);
        if rf == 0 {
            return true; // standing somewhere odd (fresh wall): let A* decide
        }
        match goal {
            Goal::Cell(c) => region_at(c) == rf,
            Goal::Touch(c) => (-1..=1).any(|dy| (-1..=1).any(|dx| region_at(c.offset(dx, dy)) == rf)),
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
            let mut room = Room { id, cells: 0, touches_edge: false };
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

    /// (current, previous) room id of a cell; previous is from before the last rebuild.
    pub fn room_ids(&self, i: usize) -> (u32, u32) {
        (self.room[i], self.prev_room[i])
    }

    /// Open floor that belongs to a room: passable and not a doorway.
    fn room_cell(&self, i: usize) -> bool {
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
