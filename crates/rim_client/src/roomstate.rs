//! What the sim knows about rooms, on the plan (DESIGN.md §6c): the look
//! never lies. The one gap that keeps a ring of walls from being a room is
//! marked, and floor walled in but beyond its roof's reach is hatched as
//! open sky. (Daylight through windows and a fire's warmth are the lighting
//! pass's.)
//!
//! Both are worked out only when rooms rebuild, and drawn over the lit
//! world so they read at night.

use macroquad::prelude::*;
use rim_sim::world::World;
use rim_sim::IVec;

/// A flood from a gap's side that runs this far is outdoors.
const FLOOD_LIMIT: usize = 2000;

#[derive(Default)]
pub struct RoomMarks {
    seen: Option<u64>,
    /// Gaps, and whether the wall they break runs east to west.
    pub gaps: Vec<(IVec, bool)>,
    /// Floor inside walls that no support roofs.
    pub sky: Vec<IVec>,
}

impl RoomMarks {
    pub fn update(&mut self, w: &World) {
        if self.seen == Some(w.map.room_rebuilds) {
            return;
        }
        self.seen = Some(w.map.room_rebuilds);
        self.gaps.clear();
        self.sky.clear();
        let m = &w.map;
        let built = |p: IVec| {
            m.inb(p)
                && !m.room_cell(m.idx(p))
                && m.fixture_at(p).and_then(|e| w.thing(e)).is_some_and(|t| !w.defs.thing(t.def).natural)
        };
        let mut seen = vec![u32::MAX; (m.w * m.h) as usize];
        let mut stack = Vec::new();
        for y in 1..m.h - 1 {
            for x in 1..m.w - 1 {
                let p = IVec::new(x, y);
                let i = m.idx(p);
                let Some(room) = m.room_at(p) else { continue };
                if !room.touches_edge {
                    if !m.covered(i) {
                        self.sky.push(p);
                    }
                    continue;
                }
                let across = built(p.offset(-1, 0)) && built(p.offset(1, 0));
                let down = built(p.offset(0, -1)) && built(p.offset(0, 1));
                if !across && !down {
                    continue;
                }
                // Blocked, would either side be cut off from the edge?
                let sides = if across { [p.offset(0, -1), p.offset(0, 1)] } else { [p.offset(-1, 0), p.offset(1, 0)] };
                let stamp = i as u32;
                let enclosed = sides.iter().any(|&s| {
                    if !m.inb(s) || !m.room_cell(m.idx(s)) {
                        return false;
                    }
                    stack.clear();
                    stack.push(s);
                    seen[m.idx(s)] = stamp;
                    let mut n = 0;
                    while let Some(q) = stack.pop() {
                        n += 1;
                        if n > FLOOD_LIMIT || q.x == 0 || q.y == 0 || q.x == m.w - 1 || q.y == m.h - 1 {
                            return false;
                        }
                        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                            let r = q.offset(dx, dy);
                            let j = m.idx(r);
                            if r == p || seen[j] == stamp || !m.room_cell(j) {
                                continue;
                            }
                            seen[j] = stamp;
                            stack.push(r);
                        }
                    }
                    true
                });
                if enclosed {
                    self.gaps.push((p, across));
                }
            }
        }
    }

    /// Draw the marks on screen, `z` points a cell.
    pub fn draw(&self, cam: &crate::Cam, (x0, y0, x1, y1): (i32, i32, i32, i32)) {
        let z = cam.zoom;
        let on = |p: IVec| p.z == cam.z && p.x >= x0 && p.x <= x1 && p.y >= y0 && p.y <= y1;
        // Open sky: a pale wash and hatching, zoomed in far enough to read.
        if z >= 8.0 {
            let step = (z / 4.0).max(4.0);
            let line = Color::new(0.59, 0.8, 1.0, 0.5);
            for &p in self.sky.iter().filter(|&&p| on(p)) {
                let (sx, sy) = cam.to_screen(p.x as f32, p.y as f32);
                draw_rectangle(sx, sy, z, z, Color::new(0.47, 0.75, 1.0, 0.12));
                // 45° strokes up and right, clipped to the cell.
                let mut k = -z + step / 2.0;
                while k < z {
                    let (dx, dy) = (k.max(0.0), (-k).max(0.0));
                    let len = (z - dx).min(z - dy);
                    if len > 0.0 {
                        draw_line(sx + dx, sy + z - dy, sx + dx + len, sy + z - dy - len, 1.0, line);
                    }
                    k += step;
                }
            }
        }
        // Gaps: a dashed red line where the wall should go on.
        let red = Color::new(1.0, 0.42, 0.35, 0.95);
        let t = (z * 0.08).clamp(1.5, 3.0);
        for &(p, across) in self.gaps.iter().filter(|(p, _)| on(*p)) {
            let (sx, sy) = cam.to_screen(p.x as f32, p.y as f32);
            let (dash, gap) = ((z * 0.14).max(3.0), (z * 0.1).max(2.0));
            let mut k = 0.0;
            while k < z {
                let e = (k + dash).min(z);
                if across {
                    draw_line(sx + k, sy + z / 2.0, sx + e, sy + z / 2.0, t, red);
                } else {
                    draw_line(sx + z / 2.0, sy + k, sx + z / 2.0, sy + e, t, red);
                }
                k = e + gap;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rim_sim::Sim;

    /// A field of 5×5 huts, each with one wall piece missing: every one is
    /// a gap, found, and the search is cheap enough to run on each rebuild.
    #[test]
    fn every_broken_hut_is_found_quickly() {
        let mut s = Sim::new(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods")), 1).unwrap();
        let d = &s.world.defs;
        let (wall, wood) = (d.thing_id("wall").unwrap(), d.thing_id("wood").unwrap());
        let o = IVec::new(20, 20);
        let mut broken = Vec::new();
        for hy in 0..10 {
            for hx in 0..10 {
                let h = o.offset(hx * 7, hy * 7);
                let open = (0..5).all(|y| (0..5).all(|x| s.world.map.passable(h.offset(x, y))));
                if !open {
                    continue;
                }
                let gap = h.offset(2, 4);
                for y in 0..5 {
                    for x in 0..5 {
                        let p = h.offset(x, y);
                        if let Some(f) = s.world.map.fixture_at(p) {
                            s.world.despawn_thing(f);
                        }
                        if (x == 0 || y == 0 || x == 4 || y == 4) && p != gap {
                            s.world.spawn_fixture_of(wall, p, false, Some(wood)).expect("a wall");
                        }
                    }
                }
                broken.push(gap);
            }
        }
        s.world.map.ensure_rooms();
        let mut marks = RoomMarks::default();
        let t0 = std::time::Instant::now();
        marks.update(&s.world);
        let ms = t0.elapsed().as_secs_f64() * 1e3;
        println!("gap search over {} huts: {ms:.3} ms", broken.len());
        assert!(broken.len() > 20, "enough huts to measure ({})", broken.len());
        for g in &broken {
            assert!(marks.gaps.iter().any(|(p, _)| p == g), "the gap at {g:?} is marked");
        }
    }
}
