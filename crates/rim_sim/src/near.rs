//! How far each cell is from the nearest terrain with a tag, for the
//! `near` term input: "how far is the nearest water" in O(1).
//!
//! One byte per cell per tag that some term reads, counted in steps that
//! may go diagonally (Chebyshev distance on the cell's own level) and capped
//! at `NEAR_CAP`. Walls don't stop it: it is how far the ground is, not how
//! far the walk is. Worked out by a breadth-first search from every tagged
//! cell at once. A terrain change is patched in a window around the cell,
//! since nothing farther than the cap away can change.

use crate::defs::DefId;
use crate::terms::NEAR_CAP;
use std::collections::VecDeque;

/// The window a patch searches: a cell within the cap of the change has its
/// nearest tagged cell within the cap of itself.
const REACH: i32 = 2 * NEAR_CAP as i32;

#[derive(Default)]
pub struct Near {
    /// By tag, then by terrain id: whether that terrain has the tag. Empty
    /// for a tag no term reads.
    tagged: Vec<Vec<bool>>,
    /// By tag: the distance per cell, every level's plane in turn. Empty
    /// for a tag no term reads.
    grids: Vec<Vec<u8>>,
    /// Cells whose tags changed since the grids were last current; `None`
    /// until the first full pass, or after too many to patch.
    dirty: Option<Vec<u32>>,
    queue: VecDeque<u32>,
    /// Cells searched by patches, for tests.
    pub patched: u64,
}

impl Near {
    /// `tagged[tag][terrain]`, with an empty list for tags no term reads.
    pub fn new(tagged: Vec<Vec<bool>>, cells: usize) -> Self {
        let grids = tagged.iter().map(|t| if t.is_empty() { Vec::new() } else { vec![NEAR_CAP; cells] }).collect();
        Near { tagged, grids, dirty: None, queue: VecDeque::new(), patched: 0 }
    }

    fn has(&self, tag: usize, terrain: DefId) -> bool {
        self.tagged[tag].get(terrain as usize).copied().unwrap_or(false)
    }

    /// The cell at `i` went from terrain `was` to `now`.
    pub fn changed(&mut self, i: usize, was: DefId, now: DefId) {
        let moved = (0..self.tagged.len()).any(|t| !self.grids[t].is_empty() && self.has(t, was) != self.has(t, now));
        if let (true, Some(d)) = (moved, &mut self.dirty) {
            d.push(i as u32);
        }
    }

    /// Distance from cell `i` to the nearest terrain tagged `tag`, in cells.
    pub fn at(&self, tag: usize, i: usize) -> u8 {
        self.grids.get(tag).and_then(|g| g.get(i)).copied().unwrap_or(NEAR_CAP)
    }

    /// Bring every grid up to date with `terrain`.
    pub fn ensure(&mut self, w: i32, h: i32, terrain: &[DefId]) {
        let plane = (w * h) as usize;
        let window = ((2 * REACH + 1) * (2 * REACH + 1)) as usize;
        match self.dirty.take() {
            Some(d) if d.is_empty() => self.dirty = Some(d),
            // Patching costs a window per cell; past a plane's worth, one
            // full pass is cheaper.
            Some(d) if d.len() * window < terrain.len() => {
                for &c in &d {
                    let (o, i) = (c as usize / plane * plane, c as i32 % (w * h));
                    let (x, y) = (i % w, i / w);
                    let lo = ((x - REACH).max(0), (y - REACH).max(0));
                    let hi = ((x + REACH).min(w - 1), (y + REACH).min(h - 1));
                    for t in 0..self.grids.len() {
                        self.search(t, w, o, terrain, lo, hi, (x, y));
                    }
                }
                self.dirty = Some(Vec::new());
            }
            _ => {
                for o in (0..terrain.len()).step_by(plane) {
                    for t in 0..self.grids.len() {
                        self.search(t, w, o, terrain, (0, 0), (w - 1, h - 1), (-1, -1));
                    }
                }
                self.dirty = Some(Vec::new());
            }
        }
    }

    /// Search the box `lo..=hi` of the plane at `o` from its tagged cells,
    /// and write what it finds: everywhere for a full pass (`around` off the
    /// map), or only within the cap of `around` for a patch, since only
    /// there is the box sure to hold each cell's nearest.
    #[allow(clippy::too_many_arguments)]
    fn search(
        &mut self,
        t: usize,
        w: i32,
        o: usize,
        terrain: &[DefId],
        lo: (i32, i32),
        hi: (i32, i32),
        around: (i32, i32),
    ) {
        if self.grids[t].is_empty() {
            return;
        }
        let whole = around.0 < 0;
        let (bw, bh) = (hi.0 - lo.0 + 1, hi.1 - lo.1 + 1);
        let at = |x: i32, y: i32| o + (y * w + x) as usize;
        // The box's own distances, so a patch never reads stale ones.
        let mut dist = vec![NEAR_CAP; (bw * bh) as usize];
        self.queue.clear();
        for y in lo.1..=hi.1 {
            for x in lo.0..=hi.0 {
                if self.has(t, terrain[at(x, y)]) {
                    let b = (y - lo.1) * bw + (x - lo.0);
                    dist[b as usize] = 0;
                    self.queue.push_back(b as u32);
                }
            }
        }
        while let Some(b) = self.queue.pop_front() {
            let d = dist[b as usize] + 1;
            if d >= NEAR_CAP {
                continue;
            }
            let (bx, by) = (b as i32 % bw, b as i32 / bw);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (nx, ny) = (bx + dx, by + dy);
                    if nx < 0 || ny < 0 || nx >= bw || ny >= bh {
                        continue;
                    }
                    let n = (ny * bw + nx) as usize;
                    if dist[n] > d {
                        dist[n] = d;
                        self.queue.push_back(n as u32);
                    }
                }
            }
        }
        if !whole {
            self.patched += (bw * bh) as u64;
        }
        let r = NEAR_CAP as i32;
        let grid = &mut self.grids[t];
        for y in lo.1..=hi.1 {
            for x in lo.0..=hi.0 {
                if whole || ((x - around.0).abs() <= r && (y - around.1).abs() <= r) {
                    grid[at(x, y)] = dist[((y - lo.1) * bw + (x - lo.0)) as usize];
                }
            }
        }
    }
}
