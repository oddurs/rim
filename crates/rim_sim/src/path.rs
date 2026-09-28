//! A* over the tile grid. Scratch buffers are generation-stamped, so a
//! search allocates nothing beyond the returned path.
//!
//! Hierarchical pathing and flow fields can slot in behind `find` later.

use crate::map::{Map, NEIGHBORS8};
use crate::world::Faction;
use crate::IVec;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Goal {
    /// Stand on this cell.
    Cell(IVec),
    /// Stand on or next to this cell (to work on it, attack it, etc.).
    Touch(IVec),
    /// Stand on or next to any cell of a thing covering `size` cells from
    /// `at`, right and down: its far side counts as much as its anchor.
    Area { at: IVec, size: [u8; 2] },
}

impl Goal {
    pub fn satisfied(self, p: IVec) -> bool {
        match self {
            Goal::Cell(c) => p == c,
            Goal::Touch(c) => p.chebyshev(c) <= 1,
            Goal::Area { .. } => p.chebyshev(self.nearest(p)) <= 1,
        }
    }
    /// The goal's cell nearest `p`: what the search heads for.
    pub fn nearest(self, p: IVec) -> IVec {
        match self {
            Goal::Cell(c) | Goal::Touch(c) => c,
            Goal::Area { at, size } => {
                IVec::new(p.x.clamp(at.x, at.x + size[0] as i32 - 1), p.y.clamp(at.y, at.y + size[1] as i32 - 1))
            }
        }
    }
    /// Every cell the goal covers (one, or a footprint).
    pub fn cells(self) -> impl Iterator<Item = IVec> {
        let (at, [w, h]) = match self {
            Goal::Cell(c) | Goal::Touch(c) => (c, [1, 1]),
            Goal::Area { at, size } => (at, size),
        };
        (0..h as i32).flat_map(move |y| (0..w as i32).map(move |x| at.offset(x, y)))
    }
}

/// A climb down or up a pit's side, as a step's cost in percent: three
/// open cells' worth.
pub const CLIMB_COST: u16 = 300;

#[derive(Default)]
pub struct Pathfinder {
    g: Vec<u32>,
    parent: Vec<u32>,
    open_gen: Vec<u32>,
    closed_gen: Vec<u32>,
    gen: u32,
    /// Open nodes by (f, h, cell): of two nodes as promising, the one
    /// nearer the goal first, so a search on open ground runs straight at
    /// it instead of widening a front of ties.
    heap: BinaryHeap<Reverse<(u32, u32, u32)>>,
    queue: VecDeque<u32>,
    pub searches: u64,
    pub expanded: u64,
    /// Searches that found no path.
    pub failed: u64,
    /// Wall-clock time spent searching, for benchmarks; never feeds the sim.
    pub micros: f64,
}

impl Pathfinder {
    /// Returns the path as a stack: `last()` is the next step. Excludes
    /// `start`. Doors `who` does not own are walls to this search.
    pub fn find(&mut self, map: &Map, start: IVec, goal: Goal, max_nodes: u32, who: Faction) -> Option<Vec<IVec>> {
        self.find_as(map, start, goal, max_nodes, who, false)
    }

    /// `find`, for a creature that climbs a pit's side or not.
    pub fn find_as(
        &mut self,
        map: &Map,
        start: IVec,
        goal: Goal,
        max_nodes: u32,
        who: Faction,
        climbs: bool,
    ) -> Option<Vec<IVec>> {
        let t = std::time::Instant::now();
        let path = self.search(map, start, goal, max_nodes, who, climbs);
        self.micros += t.elapsed().as_secs_f64() * 1e6;
        self.failed += path.is_none() as u64;
        path
    }

    /// A fresh generation: every cell unvisited, without clearing a thing.
    fn next_gen(&mut self, map: &Map) -> u32 {
        let n = map.cells();
        if self.g.len() != n {
            self.g = vec![0; n];
            self.parent = vec![0; n];
            self.open_gen = vec![0; n];
            self.closed_gen = vec![0; n];
        }
        self.gen = self.gen.wrapping_add(1);
        if self.gen == 0 {
            self.open_gen.iter_mut().for_each(|x| *x = 0);
            self.closed_gen.iter_mut().for_each(|x| *x = 0);
            self.gen = 1;
        }
        self.gen
    }

    fn search(
        &mut self,
        map: &Map,
        start: IVec,
        goal: Goal,
        max_nodes: u32,
        who: Faction,
        climbs: bool,
    ) -> Option<Vec<IVec>> {
        let gen = self.next_gen(map);
        self.searches += 1;
        self.heap.clear();

        let si = map.idx(start);
        self.g[si] = 0;
        self.parent[si] = si as u32;
        self.open_gen[si] = gen;
        let h = start.octile(goal.nearest(start));
        self.heap.push(Reverse((h, h, si as u32)));
        let mut expanded = 0;

        while let Some(Reverse((_, _, ci))) = self.heap.pop() {
            let ci = ci as usize;
            if self.closed_gen[ci] == gen {
                continue;
            }
            self.closed_gen[ci] = gen;
            let cp = map.pos(ci);
            if goal.satisfied(cp) {
                let mut path = Vec::new();
                let mut i = ci;
                while i != si {
                    path.push(map.pos(i));
                    i = self.parent[i] as usize;
                }
                self.expanded += expanded as u64;
                return Some(path);
            }
            expanded += 1;
            if expanded > max_nodes {
                break;
            }
            let cg = self.g[ci];
            let open = NEIGHBORS8.map(|(dx, dy)| map.passable_for(cp.offset(dx, dy), who));
            for (k, &(dx, dy)) in NEIGHBORS8.iter().enumerate() {
                if !open[k] {
                    continue;
                }
                let diag = k >= 4;
                // No corner cutting: both cells beside a diagonal are open
                // (NEIGHBORS8 lists +x, -x, +y, -y first).
                if diag && !(open[(dx < 0) as usize] && open[2 + (dy < 0) as usize]) {
                    continue;
                }
                let q = cp.offset(dx, dy);
                let qi = map.idx(q);
                if self.closed_gen[qi] == gen {
                    continue;
                }
                let step = if diag { 14 } else { 10 } * map.cost(q) / 100;
                let ng = cg + step.max(1);
                if self.open_gen[qi] != gen || ng < self.g[qi] {
                    self.open_gen[qi] = gen;
                    self.g[qi] = ng;
                    self.parent[qi] = ci as u32;
                    let h = q.octile(goal.nearest(q));
                    self.heap.push(Reverse((ng + h, h, qi as u32)));
                }
            }
            // Up or down the stairs (DESIGN.md §6d): one more edge, where
            // there is a portal this faction may use; and for a climber, down
            // a pit's side or up it, at a scramble's cost.
            let climb = map.climbs(ci).filter(|_| climbs).map(|qi| (qi, CLIMB_COST));
            for (qi, cost) in map.through(ci, who).into_iter().chain(climb) {
                let q = map.pos(qi);
                if self.closed_gen[qi] != gen && map.passable_for(q, who) {
                    let ng = cg + (10 * cost as u32 / 100).max(1);
                    if self.open_gen[qi] != gen || ng < self.g[qi] {
                        self.open_gen[qi] = gen;
                        self.g[qi] = ng;
                        self.parent[qi] = ci as u32;
                        let h = q.octile(goal.nearest(q));
                        self.heap.push(Reverse((ng + h, h, qi as u32)));
                    }
                }
            }
        }
        self.expanded += expanded as u64;
        None
    }

    /// Breadth-first over the cells a walker reaches from `start`, nearest
    /// (in steps) first, doors open whoever owns them. `visit` sees each
    /// cell in turn and ends the fill by returning `Some`; the fill also
    /// ends once more than `limit` cells have been reached.
    /// Visit cells `who` can walk to from `start`, nearest first, up and
    /// down stairs too, until `visit` finds something or `limit` cells.
    pub fn flood<T>(
        &mut self,
        map: &Map,
        start: IVec,
        limit: usize,
        who: Faction,
        mut visit: impl FnMut(IVec) -> Option<T>,
    ) -> Option<T> {
        let gen = self.next_gen(map);
        let mut queue = std::mem::take(&mut self.queue);
        queue.clear();
        let si = map.idx(start);
        self.closed_gen[si] = gen;
        queue.push_back(si as u32);
        let mut reached = 1;
        let mut found = None;
        while let Some(ci) = queue.pop_front() {
            let c = map.pos(ci as usize);
            found = visit(c);
            if found.is_some() || reached > limit {
                break;
            }
            let open = NEIGHBORS8.map(|(dx, dy)| map.passable(c.offset(dx, dy)));
            for (k, &(dx, dy)) in NEIGHBORS8.iter().enumerate() {
                // A diagonal past a corner is no way in: a door is only ever
                // reached straight on, so marking it from a rejected diagonal
                // would hide every room behind a door.
                if !open[k] || k >= 4 && !(open[(dx < 0) as usize] && open[2 + (dy < 0) as usize]) {
                    continue;
                }
                let qi = map.idx(c.offset(dx, dy));
                if self.closed_gen[qi] != gen {
                    self.closed_gen[qi] = gen;
                    reached += 1;
                    queue.push_back(qi as u32);
                }
            }
            // A cellar is somewhere to go (DESIGN.md §6d).
            if let Some((qi, _)) = map.through(ci as usize, who) {
                if self.closed_gen[qi] != gen {
                    self.closed_gen[qi] = gen;
                    reached += 1;
                    queue.push_back(qi as u32);
                }
            }
        }
        self.queue = queue;
        found
    }
}
