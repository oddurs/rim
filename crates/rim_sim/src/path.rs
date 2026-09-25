//! A* over the tile grid. Scratch buffers are generation-stamped, so a
//! search allocates nothing beyond the returned path.
//!
//! Hierarchical pathing and flow fields can slot in behind `find` later.

use crate::map::{Map, NEIGHBORS8};
use crate::world::Faction;
use crate::IVec;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Goal {
    /// Stand on this cell.
    Cell(IVec),
    /// Stand on or next to this cell (to work on it, attack it, etc.).
    Touch(IVec),
}

impl Goal {
    pub fn satisfied(self, p: IVec) -> bool {
        match self {
            Goal::Cell(c) => p == c,
            Goal::Touch(c) => p.chebyshev(c) <= 1,
        }
    }
    pub fn target(self) -> IVec {
        match self {
            Goal::Cell(c) | Goal::Touch(c) => c,
        }
    }
}

#[derive(Default)]
pub struct Pathfinder {
    g: Vec<u32>,
    parent: Vec<u32>,
    open_gen: Vec<u32>,
    closed_gen: Vec<u32>,
    gen: u32,
    heap: BinaryHeap<Reverse<(u32, u32, u32)>>,
    pub searches: u64,
    pub expanded: u64,
}

impl Pathfinder {
    /// Returns the path as a stack: `last()` is the next step. Excludes
    /// `start`. Doors `who` does not own are walls to this search.
    pub fn find(&mut self, map: &Map, start: IVec, goal: Goal, max_nodes: u32, who: Faction) -> Option<Vec<IVec>> {
        let n = (map.w * map.h) as usize;
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
        self.searches += 1;
        let gen = self.gen;
        let target = goal.target();
        self.heap.clear();

        let si = map.idx(start);
        self.g[si] = 0;
        self.parent[si] = si as u32;
        self.open_gen[si] = gen;
        let mut seq = 0u32;
        self.heap.push(Reverse((start.octile(target), seq, si as u32)));
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
            let open = |q: IVec| map.passable_for(q, who);
            for (k, (dx, dy)) in NEIGHBORS8.iter().enumerate() {
                let q = cp.offset(*dx, *dy);
                if !open(q) {
                    continue;
                }
                let diag = k >= 4;
                // No corner cutting.
                if diag && (!open(cp.offset(*dx, 0)) || !open(cp.offset(0, *dy))) {
                    continue;
                }
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
                    seq += 1;
                    self.heap.push(Reverse((ng + q.octile(target), seq, qi as u32)));
                }
            }
        }
        self.expanded += expanded as u64;
        None
    }
}
