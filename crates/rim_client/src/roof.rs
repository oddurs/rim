//! Roofs from far away (DESIGN.md §6c). Zoomed out, the colony reads as a
//! village: every house gets a hipped roof, worked out from the rooms the
//! sim already has, with no art.
//!
//! A house is the set of indoor rooms whose walls touch. Its roof covers
//! their cells and the walls round them. Each cell's height is its
//! Chebyshev distance to the eaves, so a rectangle gets four slopes and a
//! ridge, and an L gets a valley, whatever the shape. The material is the
//! one most of its walls are made of (`stuff.look.roof`).
//!
//! It is client-only and derived, rebuilt only when rooms are. `height` is
//! kept as data, one byte a cell, for the lighting pass to read.

use macroquad::prelude::*;
use rim_sim::world::{MadeOf, World};
use rim_sim::IVec;

/// Roofs show below this many points a cell and are whole at `FULL`.
const SHOW: f32 = 16.0;
const FULL: f32 = 10.0;

/// Past this many roofed cells on screen, a roof is drawn a row at a time
/// instead of a cell at a time.
const DETAIL_CELLS: i32 = 1500;

/// A roof covering a material names, from the fixed vocabulary, or a colour
/// written as `#rrggbb`.
fn covering(name: &str) -> Color {
    let hex = match name {
        "thatch" => "#b39a5b",
        "shingle" => "#7d5d3d",
        "turf" => "#667a41",
        "slate" => "#5b6571",
        "tile" => "#b0583f",
        other => other,
    };
    rim_sim::look::parse_rgba(hex)
        .map_or(Color::from_rgba(0xb3, 0x9a, 0x5b, 255), |[r, g, b, a]| Color::from_rgba(r, g, b, a))
}

#[derive(Default)]
pub struct Roofs {
    /// The room rebuild the roofs were worked out for.
    seen: Option<u64>,
    /// Each cell's house, plus one; 0 is open sky.
    pub house: Vec<u32>,
    /// Each roofed cell's height: its distance to the eaves, from 1.
    pub height: Vec<u8>,
    /// Each house's covering.
    color: Vec<Color>,
    /// Hearths under a roof, for chimneys.
    chimneys: Vec<IVec>,
    /// Each row's roof as runs of one house sloping one way, for drawing
    /// many roofs at once: (y, x0, x1 inclusive, house, face).
    runs: Vec<(i32, i32, i32, u32, u8)>,
    /// Where each row's runs start in `runs`, by row.
    row_start: Vec<usize>,
    /// The eaves as merged segments, in cells: (x0, y0, x1, y1).
    eaves: Vec<(i32, i32, i32, i32)>,
}

impl Roofs {
    /// Work the roofs out again if rooms were rebuilt since last time.
    pub fn update(&mut self, w: &World) {
        if self.seen == Some(w.map.room_rebuilds) {
            return;
        }
        self.seen = Some(w.map.room_rebuilds);
        let m = &w.map;
        let n = (m.w * m.h) as usize;
        self.house = vec![0; n];
        self.height = vec![0; n];
        self.chimneys.clear();
        // Indoor rooms, and what each one's roof covers: its cells and the
        // built pieces round it. Rock and water hold a roof up but take
        // none of it.
        let rooms: Vec<u32> = (1..=m.room_count() as u32).filter(|&id| m.room_by_id(id).enclosed()).collect();
        let index: std::collections::BTreeMap<u32, usize> = rooms.iter().enumerate().map(|(k, &id)| (id, k)).collect();
        let mut parent: Vec<usize> = (0..rooms.len()).collect();
        fn find(p: &mut [usize], k: usize) -> usize {
            let mut r = k;
            while p[r] != r {
                r = p[r];
            }
            p[k] = r;
            r
        }
        let built = |i: usize| m.fixture[i].and_then(|e| w.thing(e)).is_some_and(|t| !w.defs.thing(t.def).natural);
        // Which room first covers each boundary cell: a second room there
        // shares a wall, and the two are one house.
        let mut owner: Vec<usize> = vec![usize::MAX; n];
        for (k, &id) in rooms.iter().enumerate() {
            for &c in m.room_boundary(id) {
                let c = c as usize;
                if !built(c) {
                    continue;
                }
                match owner[c] {
                    usize::MAX => owner[c] = k,
                    o => {
                        let (a, b) = (find(&mut parent, o), find(&mut parent, k));
                        parent[a] = b;
                    }
                }
            }
        }
        let mut houses: std::collections::BTreeMap<usize, u32> = Default::default();
        let mut house_of = |parent: &mut Vec<usize>, k: usize| {
            let root = find(parent, k);
            let next = houses.len() as u32 + 1;
            *houses.entry(root).or_insert(next)
        };
        let mut votes: Vec<std::collections::BTreeMap<String, u32>> = Vec::new();
        for (i, &own) in owner.iter().enumerate() {
            let (room, _) = m.room_ids(i);
            let k = if room > 0 { index.get(&room).copied() } else { None }.or((own != usize::MAX).then_some(own));
            let Some(k) = k else { continue };
            let h = house_of(&mut parent, k);
            self.house[i] = h;
            if votes.len() < h as usize {
                votes.resize(h as usize, Default::default());
            }
            // A wall's material is a vote for the covering.
            if own != usize::MAX {
                let roof = m.fixture[i]
                    .and_then(|e| w.ecs.get::<&MadeOf>(e).ok().map(|mo| mo.0))
                    .and_then(|mat| w.defs.thing(mat).stuff.as_ref())
                    .and_then(|st| st.look.roof.clone());
                if let Some(r) = roof {
                    *votes[h as usize - 1].entry(r).or_default() += 1;
                }
            }
            if room > 0 {
                if let Some(t) = m.fixture[i].and_then(|e| w.thing(e)) {
                    if w.defs.thing(t.def).tags.iter().any(|g| g == "fire") && t.pos == m.pos(i) {
                        self.chimneys.push(t.pos);
                    }
                }
            }
        }
        self.color = votes
            .iter()
            .map(|v| {
                covering(
                    v.iter()
                        .max_by_key(|(name, c)| (**c, std::cmp::Reverse((*name).clone())))
                        .map_or("thatch", |(n, _)| n),
                )
            })
            .collect();
        // Height: rings in from the eaves, where a neighbour is another
        // house or open sky.
        let same = |i: usize, q: IVec| m.inb(q) && self.house[m.idx(q)] == self.house[i];
        let mut ring: Vec<usize> = Vec::new();
        for i in 0..n {
            if self.house[i] == 0 {
                continue;
            }
            let p = m.pos(i);
            if rim_sim::map::NEIGHBORS8.iter().any(|&(dx, dy)| !same(i, p.offset(dx, dy))) {
                self.height[i] = 1;
                ring.push(i);
            }
        }
        let mut level = 1u8;
        while !ring.is_empty() {
            level = level.saturating_add(1);
            let mut next = Vec::new();
            for &i in &ring {
                let p = m.pos(i);
                for &(dx, dy) in &rim_sim::map::NEIGHBORS8 {
                    let q = p.offset(dx, dy);
                    if same(i, q) {
                        let j = m.idx(q);
                        if self.height[j] == 0 {
                            self.height[j] = level;
                            next.push(j);
                        }
                    }
                }
            }
            ring = next;
        }
        self.build_runs(w);
    }

    /// The whole-cell slope of each roofed cell, merged into runs along each
    /// row, and the eaves merged into segments.
    fn build_runs(&mut self, w: &World) {
        let m = &w.map;
        self.runs.clear();
        self.eaves.clear();
        self.row_start = Vec::with_capacity(m.h as usize + 1);
        for y in 0..m.h {
            self.row_start.push(self.runs.len());
            let mut x = 0;
            while x < m.w {
                let i = m.idx(IVec::new(x, y));
                let h = self.house[i];
                if h == 0 {
                    x += 1;
                    continue;
                }
                let face = self.face(w, x, y);
                let x0 = x;
                while x + 1 < m.w && {
                    let j = m.idx(IVec::new(x + 1, y));
                    self.house[j] == h && self.face(w, x + 1, y) == face
                } {
                    x += 1;
                }
                self.runs.push((y, x0, x, h, face));
                x += 1;
            }
        }
        self.row_start.push(self.runs.len());
        // Eaves: a side where the house ends, merged along rows and columns.
        let edge = |x: i32, y: i32, dx: i32, dy: i32| {
            let p = IVec::new(x, y);
            let h = self.house[m.idx(p)];
            let q = p.offset(dx, dy);
            h != 0 && (!m.inb(q) || self.house[m.idx(q)] != h)
        };
        for (dy, yo) in [(-1, 0), (1, 1)] {
            for y in 0..m.h {
                let mut x = 0;
                while x < m.w {
                    if !edge(x, y, 0, dy) {
                        x += 1;
                        continue;
                    }
                    let x0 = x;
                    while x + 1 < m.w && edge(x + 1, y, 0, dy) {
                        x += 1;
                    }
                    self.eaves.push((x0, y + yo, x + 1, y + yo));
                    x += 1;
                }
            }
        }
        for (dx, xo) in [(-1, 0), (1, 1)] {
            for x in 0..m.w {
                let mut y = 0;
                while y < m.h {
                    if !edge(x, y, dx, 0) {
                        y += 1;
                        continue;
                    }
                    let y0 = y;
                    while y + 1 < m.h && edge(x, y + 1, dx, 0) {
                        y += 1;
                    }
                    self.eaves.push((x + xo, y0, x + xo, y + 1));
                    y += 1;
                }
            }
        }
    }

    /// Which way a roofed cell slopes as a whole: 0 north, 1 west, 2 east,
    /// 3 south, 4 flat (a ridge's peak).
    fn face(&self, w: &World, x: i32, y: i32) -> u8 {
        let m = &w.map;
        let h = self.house[m.idx(IVec::new(x, y))];
        let at = |dx: i32, dy: i32| {
            let q = IVec::new(x + dx, y + dy);
            if m.inb(q) && self.house[m.idx(q)] == h {
                self.height[m.idx(q)] as i32
            } else {
                0
            }
        };
        let (gx, gy) = (at(1, 0) - at(-1, 0), at(0, 1) - at(0, -1));
        if gx == 0 && gy == 0 {
            4
        } else if gy.abs() >= gx.abs() {
            if gy > 0 {
                0
            } else {
                3
            }
        } else if gx > 0 {
            1
        } else {
            2
        }
    }

    /// How opaque roofs are at zoom `z` (points a cell): none close in.
    pub fn alpha(z: f32) -> f32 {
        ((SHOW - z) / (SHOW - FULL)).clamp(0.0, 1.0)
    }

    /// Draw every roof on screen but `lifted`'s, at `alpha`, tinted by the
    /// outdoor light `tint`, from one light in the north-west.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        w: &World,
        cam: &crate::Cam,
        (x0, y0, x1, y1): (i32, i32, i32, i32),
        alpha: f32,
        lifted: u32,
        tint: Vec3,
    ) {
        if alpha <= 0.0 || self.house.is_empty() {
            return;
        }
        let m = &w.map;
        let z = cam.zoom;
        let lit = |c: Color, f: f32| {
            Color::new((c.r * f * tint.x).min(1.0), (c.g * f * tint.y).min(1.0), (c.b * f * tint.z).min(1.0), alpha)
        };
        // Brightness by which way a slope faces: N W E S, and flat.
        const FACE: [f32; 5] = [1.14, 1.02, 0.86, 0.74, 1.0];
        let rows = (y0.max(0) as usize, (y1 + 1).clamp(0, m.h) as usize);
        let visible = || {
            self.runs[self.row_start[rows.0]..self.row_start[rows.1]]
                .iter()
                .filter(move |r| r.2 >= x0 && r.1 <= x1 && r.3 != lifted)
        };
        let cells: i32 = visible().map(|r| r.2.min(x1) - r.1.max(x0) + 1).sum();
        // Many roofs on screen: a rectangle a run, each sloping one way, and
        // merged eaves. Few, close in: every cell's hips and courses.
        if cells > DETAIL_CELLS {
            for &(y, a, b, _, _) in visible() {
                let (sx, sy) = cam.to_screen(a as f32 + 0.3, y as f32 + 0.42);
                draw_rectangle(
                    sx,
                    sy,
                    (b - a + 1) as f32 * z + 0.5,
                    z + 0.5,
                    Color::new(0.03, 0.04, 0.02, 0.3 * alpha),
                );
            }
            for &(y, a, b, h, face) in visible() {
                let (sx, sy) = cam.to_screen(a as f32, y as f32);
                draw_rectangle(
                    sx,
                    sy,
                    (b - a + 1) as f32 * z + 0.5,
                    z + 0.5,
                    lit(self.color[h as usize - 1], FACE[face as usize]),
                );
            }
            let ink = Color::new(0.09, 0.07, 0.05, 0.8 * alpha);
            let t = (z * 0.06).clamp(1.0, 2.0);
            for &(ax, ay, bx, by) in &self.eaves {
                if bx < x0 || ax > x1 + 1 || by < y0 || ay > y1 + 1 {
                    continue;
                }
                let (p, q) = (cam.to_screen(ax as f32, ay as f32), cam.to_screen(bx as f32, by as f32));
                draw_line(p.0, p.1, q.0, q.1, t, ink);
            }
            return;
        }
        let hat = |x: i32, y: i32| {
            let p = IVec::new(x, y);
            if !m.inb(p) {
                return (0, 0u8);
            }
            let i = m.idx(p);
            (self.house[i], self.height[i])
        };
        // The roof's shadow on the ground, down and away from the light.
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (h, _) = hat(x, y);
                if h == 0 || h == lifted {
                    continue;
                }
                let (sx, sy) = cam.to_screen(x as f32 + 0.3, y as f32 + 0.42);
                draw_rectangle(sx, sy, z + 0.5, z + 0.5, Color::new(0.03, 0.04, 0.02, 0.3 * alpha));
            }
        }
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (h, hc) = hat(x, y);
                if h == 0 || h == lifted {
                    continue;
                }
                let c = self.color[h as usize - 1];
                let at = |dx: i32, dy: i32| {
                    let (hn, v) = hat(x + dx, y + dy);
                    if hn == h {
                        v
                    } else {
                        0
                    }
                };
                // The way the whole cell slopes, for triangles on the level.
                let (gx, gy) = (at(1, 0) as i32 - at(-1, 0) as i32, at(0, 1) as i32 - at(0, -1) as i32);
                let whole = if gx == 0 && gy == 0 {
                    4
                } else if gy.abs() >= gx.abs() {
                    if gy > 0 {
                        0
                    } else {
                        3
                    }
                } else if gx > 0 {
                    1
                } else {
                    2
                };
                let (sx, sy) = cam.to_screen(x as f32, y as f32);
                let (mx, my) = (sx + z / 2.0, sy + z / 2.0);
                let corners = [(sx, sy), (sx + z, sy), (sx + z, sy + z), (sx, sy + z)];
                // Triangles N E S W, each from the middle to one side, shaded
                // by the slope toward that side's neighbour.
                for (k, (dx, dy, down, up)) in
                    [(0, -1, 0, 3), (1, 0, 2, 1), (0, 1, 3, 0), (-1, 0, 1, 2)].into_iter().enumerate()
                {
                    let hn = at(dx, dy);
                    let face = if hn < hc {
                        down
                    } else if hn > hc {
                        up
                    } else {
                        whole
                    };
                    let (a, b) = (corners[k], corners[(k + 1) % 4]);
                    let pad = 0.4;
                    let grow = |(px, py): (f32, f32)| (px + (px - mx).signum() * pad, py + (py - my).signum() * pad);
                    let (a, b) = (grow(a), grow(b));
                    draw_triangle(
                        vec2(mx, my),
                        vec2(a.0, a.1),
                        vec2(b.0, b.1),
                        lit(c, FACE[face] * (0.97 + 0.05 * hash(x, y))),
                    );
                }
                // The covering's courses, parallel to the eaves.
                if z >= 6.0 {
                    let line = lit(c, 0.65);
                    let along_x = matches!(whole, 0 | 3);
                    for k in 1..3 {
                        let f = k as f32 / 3.0;
                        if along_x {
                            draw_line(sx, sy + f * z, sx + z, sy + f * z, 1.0, Color { a: 0.45 * alpha, ..line });
                        } else {
                            draw_line(sx + f * z, sy, sx + f * z, sy + z, 1.0, Color { a: 0.45 * alpha, ..line });
                        }
                    }
                }
                // Eaves: a line where the roof ends.
                let ink = Color::new(0.09, 0.07, 0.05, 0.8 * alpha);
                let t = (z * 0.06).clamp(1.0, 2.0);
                if at(0, -1) == 0 {
                    draw_line(sx, sy, sx + z, sy, t, ink);
                }
                if at(0, 1) == 0 {
                    draw_line(sx, sy + z, sx + z, sy + z, t, ink);
                }
                if at(-1, 0) == 0 {
                    draw_line(sx, sy, sx, sy + z, t, ink);
                }
                if at(1, 0) == 0 {
                    draw_line(sx + z, sy, sx + z, sy + z, t, ink);
                }
            }
        }
        // A chimney over every hearth.
        for &p in &self.chimneys {
            let (h, _) = hat(p.x, p.y);
            if h == lifted || p.x < x0 || p.x > x1 || p.y < y0 || p.y > y1 {
                continue;
            }
            let (sx, sy) = cam.to_screen(p.x as f32 + 0.3, p.y as f32 + 0.3);
            let s = z * 0.4;
            draw_rectangle(sx + s * 0.4, sy + s * 0.6, s, s, Color::new(0.03, 0.04, 0.02, 0.35 * alpha));
            draw_rectangle(sx, sy, s, s, lit(Color::from_rgba(0x4a, 0x44, 0x40, 255), 1.0));
            draw_rectangle(
                sx + s * 0.25,
                sy + s * 0.25,
                s * 0.5,
                s * 0.5,
                lit(Color::from_rgba(0x1e, 0x1a, 0x18, 255), 1.0),
            );
        }
    }

    /// The house over `p`, plus one; 0 for none.
    pub fn house_at(&self, w: &World, p: IVec) -> u32 {
        if !w.map.inb(p) || self.house.is_empty() {
            return 0;
        }
        self.house[w.map.idx(p)]
    }
}

fn hash(x: i32, y: i32) -> f32 {
    rim_sim::rng::hash2_f(x as i64, y as i64, 11) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use rim_sim::Sim;

    /// An L of wood walls: one house, one roof over both arms, rising to a
    /// ridge in each and meeting in a valley at the corner.
    #[test]
    fn an_l_shaped_house_gets_one_roof() {
        let mut s = Sim::new(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods")), 1).unwrap();
        let o = s.world.colony_center().unwrap().offset(8, 8);
        let d = &s.world.defs;
        let (wall, wood) = (d.thing_id("wall").unwrap(), d.thing_id("wood").unwrap());
        // Cells of the L's floor: an 8×3 bar and a 3×5 leg down its west end.
        let floor =
            |x: i32, y: i32| (1..9).contains(&x) && (1..4).contains(&y) || (1..4).contains(&x) && (1..9).contains(&y);
        for y in -1..=10 {
            for x in -1..=10 {
                if let Some(f) = s.world.map.fixture_at(o.offset(x, y)) {
                    s.world.despawn_thing(f);
                }
            }
        }
        for y in 0..=9 {
            for x in 0..=9 {
                let near = (-1..=1).any(|dy| (-1..=1).any(|dx| floor(x + dx, y + dy)));
                if near && !floor(x, y) {
                    s.world.spawn_fixture_of(wall, o.offset(x, y), false, Some(wood)).expect("a wall");
                }
            }
        }
        s.world.map.ensure_rooms();
        let mut r = Roofs::default();
        r.update(&s.world);
        let h = r.house_at(&s.world, o.offset(2, 2));
        assert!(h > 0, "the L is roofed");
        assert_eq!(r.house_at(&s.world, o.offset(7, 2)), h, "the bar's far end is the same roof");
        assert_eq!(r.house_at(&s.world, o.offset(2, 7)), h, "so is the leg's");
        let height = |x, y| r.height[s.world.map.idx(o.offset(x, y))];
        assert_eq!(height(0, 5), 1, "eaves over the walls");
        assert!(height(2, 6) > height(1, 6), "a slope up to the leg's ridge");
        assert_eq!(covering("shingle"), covering("#7d5d3d"), "a name or a colour");
    }
}
