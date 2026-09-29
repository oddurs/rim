//! A* over the tile grid. Scratch buffers are generation-stamped, so a
//! search allocates nothing beyond the returned path.
//!
//! The heuristic is the larger of the octile distance and a landmark bound
//! (`Landmarks`): around a lake the octile distance points through the
//! water, and a search floods the whole near shore before it finds the way
//! round; the landmarks know the lake is there.
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

/// Landmarks at the map's edge: its corners and the middles of its sides.
const LANDMARKS: usize = 8;
/// Not reached from a landmark.
const FAR: u16 = u16::MAX;

/// Ticks from a change to the land until the landmarks built from it are
/// used (`World::follow_land`): they're built on another thread meanwhile,
/// and searches use the octile distance alone.
pub const LAND_DELAY: u64 = 120;

/// What the landmarks read of the map, copied so they can be built off the
/// sim's thread: each cell's `Map::land` (0 for lake) and the portals.
pub struct Land {
    w: usize,
    plane: usize,
    cost: Vec<u16>,
    /// (cell, the other end, cost), both ways round, in cell order.
    links: Vec<(u32, u32, u16)>,
}

impl Land {
    pub fn of(map: &Map) -> Land {
        let cost = (0..map.cells()).map(|i| map.land(i).map_or(0, |c| c.clamp(1, u16::MAX as u32) as u16)).collect();
        let mut links: Vec<(u32, u32, u16)> = map
            .portals()
            .iter()
            .flat_map(|p| {
                let (t, b) = (map.idx(p.top) as u32, map.idx(p.bottom) as u32);
                [(t, b, p.cost), (b, t, p.cost)]
            })
            .collect();
        links.sort_unstable();
        Land { w: map.w as usize, plane: map.plane(), cost, links }
    }
}

/// Distances from a few cells at the map's edge over the land alone
/// (`Map::land`): lakes in the way, each step at its terrain's cost, and
/// everything that comes and goes left out (walls, doors, things, snow,
/// mud, other pawns), with levels joined only by their portals. A real
/// path costs at least as much, so for any landmark L, |d(L, goal) −
/// d(L, n)| bounds the way from n to the goal from below (the triangle
/// inequality) and consistently: A* stays optimal (ALT: Goldberg and
/// Harrelson, 2005). The exception is a floor cheaper than open ground
/// (core's is 70), which counts as open ground here, as the octile
/// distance counts every step: over one the two are off alike (3cb3dd5f).
///
/// They depend only on the land and the portals, which bump
/// `Map::land_rev`: `World::follow_land` builds them again from a change,
/// and they're used from exactly `LAND_DELAY` ticks later, in a saved game
/// as in the one that never saved. Distances are u16, as a step costs at
/// most a few hundred; one past that saturates, which only weakens the
/// bound.
pub struct Landmarks {
    /// Per landmark: the way from it to each cell, and from each cell to it.
    from: Vec<Vec<u16>>,
    to: Vec<Vec<u16>>,
}

impl Landmarks {
    pub fn build(land: &Land) -> Landmarks {
        let (w, h) = (land.w as i32, (land.plane / land.w) as i32);
        let edge: [(i32, i32); LANDMARKS] =
            [(0, 0), (w / 2, 0), (w - 1, 0), (w - 1, h / 2), (w - 1, h - 1), (w / 2, h - 1), (0, h - 1), (0, h / 2)];
        let marks: Vec<usize> = edge.iter().filter_map(|&(x, y)| Self::dry_near(land, x, y)).collect();
        Landmarks {
            from: marks.iter().map(|&l| Self::dijkstra(land, l, false)).collect(),
            to: marks.iter().map(|&l| Self::dijkstra(land, l, true)).collect(),
        }
    }

    /// The land cell nearest (x, y) on the surface, ring by ring.
    fn dry_near(land: &Land, x: i32, y: i32) -> Option<usize> {
        let (w, h) = (land.w as i32, (land.plane / land.w) as i32);
        (0..w.max(h)).find_map(|r| {
            (-r..=r)
                .flat_map(|dy| (-r..=r).map(move |dx| (x + dx, y + dy)))
                .filter(|&(qx, qy)| {
                    (qx - x).abs().max((qy - y).abs()) == r && (0..w).contains(&qx) && (0..h).contains(&qy)
                })
                .map(|(qx, qy)| (qy * w + qx) as usize)
                .find(|&i| land.cost[i] > 0)
        })
    }

    /// Dijkstra over the land from cell `l`, or with `to`, from every cell
    /// to `l` (the steps walked backwards). A step costs what `search`
    /// charges for it at the land's cost of the cell stepped into; the way
    /// through a portal, the portal's.
    fn dijkstra(land: &Land, l: usize, to: bool) -> Vec<u16> {
        let (w, plane) = (land.w, land.plane);
        let h = plane / w;
        let mut dist = vec![u32::MAX; land.cost.len()];
        let mut heap = BinaryHeap::new();
        dist[l] = 0;
        heap.push(Reverse((0u32, l as u32)));
        while let Some(Reverse((d, i))) = heap.pop() {
            let i = i as usize;
            if d > dist[i] {
                continue;
            }
            let (base, at) = (i - i % plane, i % plane);
            let (x, y) = ((at % w) as i32, (at / w) as i32);
            let mut relax = |j: usize, step: u32| {
                let nd = d + step;
                if nd < dist[j] {
                    dist[j] = nd;
                    heap.push(Reverse((nd, j as u32)));
                }
            };
            for (k, &(dx, dy)) in NEIGHBORS8.iter().enumerate() {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let j = base + ny as usize * w + nx as usize;
                // Forwards the step into j costs j's land; walked backwards,
                // the step from j into i costs i's.
                let c = if to { land.cost[i] } else { land.cost[j] } as u32;
                if land.cost[j] > 0 {
                    relax(j, ((if k >= 4 { 14 } else { 10 }) * c / 100).max(1));
                }
            }
            let from = land.links.partition_point(|&(a, _, _)| (a as usize) < i);
            for &(_, j, cost) in land.links[from..].iter().take_while(|&&(a, _, _)| a as usize == i) {
                if land.cost[j as usize] > 0 {
                    relax(j as usize, (10 * cost as u32 / 100).max(1));
                }
            }
        }
        dist.into_iter().map(|d| if d == u32::MAX { FAR } else { d.min(FAR as u32 - 1) as u16 }).collect()
    }

    /// Each landmark's way to and from cell `t`, for a search's goal.
    fn at(&self, t: usize) -> Goalward {
        let mut out = Goalward { from: [FAR; LANDMARKS], to: [FAR; LANDMARKS] };
        for (o, d) in out.from.iter_mut().zip(&self.from) {
            *o = d[t];
        }
        for (o, d) in out.to.iter_mut().zip(&self.to) {
            *o = d[t];
        }
        out
    }

    /// The bound on the way from cell `n` to a goal `t`, by the triangle
    /// inequality each way round: d(L, t) ≤ d(L, n) + d(n, t), and
    /// d(n, L) ≤ d(n, t) + d(t, L). A cell a landmark doesn't reach bounds
    /// nothing from it.
    #[inline]
    fn bound(&self, n: usize, t: &Goalward) -> u32 {
        let mut best = 0;
        for (d, &lt) in self.from.iter().zip(&t.from) {
            let ln = d[n];
            if ln != FAR && lt != FAR && lt > ln {
                best = best.max((lt - ln) as u32);
            }
        }
        for (d, &tl) in self.to.iter().zip(&t.to) {
            let nl = d[n];
            if nl != FAR && tl != FAR && nl > tl {
                best = best.max((nl - tl) as u32);
            }
        }
        best
    }

    /// Bytes the tables take.
    pub fn bytes(&self) -> usize {
        self.from.iter().chain(&self.to).map(|d| d.len() * 2).sum()
    }
}

/// A goal cell's distances from and to each landmark.
#[derive(Clone, Copy)]
struct Goalward {
    from: [u16; LANDMARKS],
    to: [u16; LANDMARKS],
}

/// The next landmarks, while `World::follow_land` waits to switch to them.
enum Next {
    Building(std::thread::JoinHandle<Landmarks>),
    Built(Landmarks),
}

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
    /// The landmarks searches use, when they're current (`Landmarks`).
    marks: Option<Landmarks>,
    next: Option<Next>,
    /// The `Map::land_rev` last seen.
    seen: u64,
    pub searches: u64,
    pub expanded: u64,
    /// Searches that found no path.
    pub failed: u64,
    /// Wall-clock time spent searching, for benchmarks; never feeds the sim.
    pub micros: f64,
}

impl Pathfinder {
    /// Build the landmarks from the map as it is, now, outside any tick: a
    /// new game's, or a loaded one's. `pending`: the land changed less than
    /// `LAND_DELAY` ticks before the save, so the game that never saved
    /// isn't using them yet, and neither is this one until `switch`.
    pub fn settle(&mut self, map: &Map, pending: bool) {
        self.seen = map.land_rev();
        let built = Landmarks::build(&Land::of(map));
        if pending {
            (self.marks, self.next) = (None, Some(Next::Built(built)));
        } else {
            (self.marks, self.next) = (Some(built), None);
        }
    }

    /// Whether the land changed since last asked. If it did, the landmarks
    /// stop, and new ones start building from the land as it is now; a
    /// build still running from an earlier change is left to finish unread.
    pub fn land_moved(&mut self, map: &Map) -> bool {
        if map.land_rev() == self.seen {
            return false;
        }
        self.seen = map.land_rev();
        self.marks = None;
        let land = Land::of(map);
        self.next = Some(Next::Building(std::thread::spawn(move || Landmarks::build(&land))));
        true
    }

    /// Use the landmarks built since the land last changed, waiting for
    /// them if they aren't done.
    pub fn switch(&mut self) {
        self.marks = match self.next.take() {
            Some(Next::Building(h)) => h.join().ok(),
            Some(Next::Built(m)) => Some(m),
            None => None,
        };
    }

    /// The landmarks searches use now, if any.
    pub fn landmarks(&self) -> Option<&Landmarks> {
        self.marks.as_ref()
    }

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
        // A climber's ways down a pit's side aren't in the landmarks'
        // land, so its searches keep the octile distance alone. A goal of
        // one cell has the same landmark distances all search long.
        let marks = self.marks.as_ref().filter(|_| !climbs);
        let one = match goal {
            Goal::Cell(c) | Goal::Touch(c) => marks.map(|m| m.at(map.idx(c))),
            Goal::Area { .. } => None,
        };
        let heuristic = |q: IVec| {
            let t = goal.nearest(q);
            let Some(m) = marks else { return q.octile(t) };
            let at = one.unwrap_or_else(|| m.at(map.idx(t)));
            m.bound(map.idx(q), &at).max(q.octile(t))
        };
        let h = heuristic(start);
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
                    let h = heuristic(q);
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
                        let h = heuristic(q);
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
