//! Material patterns (DESIGN.md §6c): how a wall or floor shows what it is
//! made of, in hairlines and flecks, with no art. A pattern is laid out in
//! world space and along the run its cell belongs to, so courses of stone
//! run on from cell to cell and turn with the wall. It stays inside the
//! mass: clear of the outline on open sides, and off the rounded corners.
//!
//! Zoomed out, where hairlines would be noise and the chunk count is at its
//! highest, patterns fade and then aren't drawn at all.

use crate::draw::{joins, Join, Sink};
use macroquad::prelude::Color;
use rim_sim::look::Pattern;
use rim_sim::rng::hash2_f;
use rim_sim::world::World;
use rim_sim::IVec;

/// Below this many points a cell, no pattern; above `FULL`, a full one.
const FADE: f32 = 14.0;
const FULL: f32 = 22.0;

/// How far a pattern keeps from an open side, as a fraction of the cell,
/// so it doesn't touch the outline.
const MARGIN: f32 = 0.07;

fn shade(c: Color, f: f32, a: f32) -> Color {
    Color::new((c.r * f).min(1.0), (c.g * f).min(1.0), (c.b * f).min(1.0), c.a * a)
}

fn h(x: i32, y: i32, k: u64) -> f32 {
    hash2_f(x as i64, y as i64, k) as f32
}

/// The cell being patterned: which way its run goes, where the pattern
/// may reach, and how to put a point in run space on the screen.
struct Cell {
    /// Run space: `u` along the run, `v` across, in world cells.
    along_y: bool,
    u0: f32,
    v0: f32,
    at: (f32, f32),
    z: f32,
    /// Where marks may go, in the cell's own unit square.
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    /// Rounded outer corners, NW NE SE SW, and their radius.
    outer: [bool; 4],
    r: f32,
}

impl Cell {
    /// Run space to the cell's unit square.
    fn unit(&self, u: f32, v: f32) -> (f32, f32) {
        let (a, b) = (u - self.u0, v - self.v0);
        if self.along_y {
            (b, a)
        } else {
            (a, b)
        }
    }

    fn screen(&self, (x, y): (f32, f32)) -> (f32, f32) {
        (self.at.0 + x * self.z, self.at.1 + y * self.z)
    }

    /// Inside the mass: within the margins and off a rounded corner.
    fn inside(&self, (x, y): (f32, f32)) -> bool {
        if x < self.x0 || x > self.x1 || y < self.y0 || y > self.y1 {
            return false;
        }
        let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        for (k, &(cx, cy)) in corners.iter().enumerate() {
            if !self.outer[k] || self.r <= 0.0 {
                continue;
            }
            // The corner's round, inset like the margin.
            let r = self.r - MARGIN * 0.5;
            let (ox, oy) = (if cx == 0.0 { r } else { 1.0 - r }, if cy == 0.0 { r } else { 1.0 - r });
            let beyond = (if cx == 0.0 { x < ox } else { x > ox }) && (if cy == 0.0 { y < oy } else { y > oy });
            if beyond && (x - ox).hypot(y - oy) > r - MARGIN {
                return false;
            }
        }
        true
    }

    /// A segment in run space, cut to what lies inside, drawn.
    fn line(&self, s: &mut impl Sink, (u0, v0): (f32, f32), (u1, v1): (f32, f32), t: f32, c: Color) {
        let (a, b) = (self.unit(u0, v0), self.unit(u1, v1));
        let Some((a, b)) = clip(a, b, (self.x0, self.y0, self.x1, self.y1)) else { return };
        // Pull an end off a rounded corner toward the other end.
        let pull = |from: (f32, f32), to: (f32, f32)| -> Option<(f32, f32)> {
            if self.inside(from) {
                return Some(from);
            }
            let (mut lo, mut hi) = (0.0f32, 1.0f32);
            if !self.inside(to) {
                return None;
            }
            for _ in 0..8 {
                let m = (lo + hi) / 2.0;
                let p = (from.0 + (to.0 - from.0) * m, from.1 + (to.1 - from.1) * m);
                if self.inside(p) {
                    hi = m;
                } else {
                    lo = m;
                }
            }
            Some((from.0 + (to.0 - from.0) * hi, from.1 + (to.1 - from.1) * hi))
        };
        let (Some(a2), Some(b2)) = (pull(a, b), pull(b, a)) else { return };
        let ((x0, y0), (x1, y1)) = (self.screen(a2), self.screen(b2));
        s.line(x0, y0, x1, y1, t, c);
    }

    fn dot(&self, s: &mut impl Sink, (u, v): (f32, f32), r: f32, c: Color) {
        let p = self.unit(u, v);
        if self.inside(p) {
            let (x, y) = self.screen(p);
            s.rect(x - r, y - r, 2.0 * r, 2.0 * r, c);
        }
    }

    /// A small block, filled and outlined, if its middle is inside.
    fn block(&self, s: &mut impl Sink, (u, v): (f32, f32), (du, dv): (f32, f32), fill: Color, line: Color, t: f32) {
        if !self.inside(self.unit(u, v)) {
            return;
        }
        let (a, b) = (self.unit(u - du / 2.0, v - dv / 2.0), self.unit(u + du / 2.0, v + dv / 2.0));
        let (x0, y0) = (a.0.min(b.0).max(self.x0), a.1.min(b.1).max(self.y0));
        let (x1, y1) = (a.0.max(b.0).min(self.x1), a.1.max(b.1).min(self.y1));
        let ((sx0, sy0), (sx1, sy1)) = (self.screen((x0, y0)), self.screen((x1, y1)));
        s.rect(sx0, sy0, sx1 - sx0, sy1 - sy0, fill);
        for ((ax, ay), (bx, by)) in
            [((sx0, sy0), (sx1, sy0)), ((sx1, sy0), (sx1, sy1)), ((sx1, sy1), (sx0, sy1)), ((sx0, sy1), (sx0, sy0))]
        {
            s.line(ax, ay, bx, by, t, line);
        }
    }
}

/// Liang–Barsky: the part of a segment inside a rectangle.
fn clip(a: (f32, f32), b: (f32, f32), (x0, y0, x1, y1): (f32, f32, f32, f32)) -> Option<((f32, f32), (f32, f32))> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (p, q) in [(-dx, a.0 - x0), (dx, x1 - a.0), (-dy, a.1 - y0), (dy, y1 - a.1)] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    (t0 <= t1).then_some(((a.0 + dx * t0, a.1 + dy * t0), (a.0 + dx * t1, a.1 + dy * t1)))
}

/// Draw `pat` over the footprint at `at`, `z` points a cell, in `c` (the
/// thing's colour). Walls follow their run; a thing that joins nothing is
/// patterned east to west.
#[allow(clippy::too_many_arguments)]
pub fn paint(
    s: &mut impl Sink,
    w: &World,
    p: IVec,
    span: [u32; 2],
    join: Join,
    pat: Pattern,
    at: (f32, f32),
    z: f32,
    c: Color,
) {
    if pat == Pattern::None || z < FADE || span != [1, 1] {
        return;
    }
    let fade = ((z - FADE) / (FULL - FADE)).clamp(0.0, 1.0);
    let [n, e, so, wv] = [(0, -1), (1, 0), (0, 1), (-1, 0)].map(|(dx, dy)| joins(w, p.offset(dx, dy), join));
    let along_y = (n || so) && !(e || wv);
    let (u0, v0) = if along_y { (p.y as f32, p.x as f32) } else { (p.x as f32, p.y as f32) };
    // On a joined side the neighbour's fill reaches half a point over this
    // cell, and is painted after it: reach back as far, so a course carries
    // across the joint without a notch.
    let over = 0.5 / z;
    let m = |joined: bool| if joined { -over } else { MARGIN };
    let cell = Cell {
        along_y,
        u0,
        v0,
        at,
        z,
        x0: m(wv),
        y0: m(n),
        x1: 1.0 - m(e).max(0.0),
        y1: 1.0 - m(so).max(0.0),
        outer: [!n && !wv, !n && !e, !so && !e, !so && !wv],
        r: join.map_or(0.0, |(_, r)| r),
    };
    // Which ends of the run are open, along `u`: for log ends.
    let (lo_end, hi_end) = if along_y { (!n, !so) } else { (!wv, !e) };
    let t = (z * 0.03).clamp(0.6, 1.4);
    let dark = shade(c, 0.6, 0.85 * fade);
    let light = shade(c, 1.3, 0.6 * fade);
    let (u, v) = (u0, v0);
    let (x, y) = (p.x, p.y);
    // Courses: rows across the run, joints staggered row by row.
    let courses = |s: &mut dyn FnMut((f32, f32), (f32, f32)), rows: i32, len: f32, stagger: f32, v_rows: f32| {
        for k in 0..rows {
            let vk = v + k as f32 / rows as f32;
            if k > 0 {
                s((u, vk), (u + 1.0, vk));
            }
            let off = ((v_rows as i32 * rows + k).rem_euclid(3)) as f32 * stagger;
            let mut n = ((u - off) / len).floor() as i32 - 1;
            loop {
                let uj = n as f32 * len + off;
                if uj >= u + 1.0 {
                    break;
                }
                if uj > u + 0.01 {
                    s((uj, vk), (uj, vk + 1.0 / rows as f32));
                }
                n += 1;
            }
        }
    };
    match pat {
        Pattern::None => {}
        Pattern::Courses => courses(&mut |a, b| cell.line(s, a, b, t, dark), 3, 0.72, 0.36, v),
        Pattern::Bond => courses(&mut |a, b| cell.line(s, a, b, t, shade(c, 1.55, 0.75 * fade)), 5, 0.44, 0.22, v),
        Pattern::Planks => courses(&mut |a, b| cell.line(s, a, b, t, shade(c, 0.7, 0.55 * fade)), 4, 1.6, 0.53, v),
        Pattern::Flags => courses(&mut |a, b| cell.line(s, a, b, t, shade(c, 0.7, 0.55 * fade)), 2, 0.8, 0.4, v),
        Pattern::Logs => {
            for k in 1..3 {
                let vk = v + k as f32 / 3.0;
                cell.line(s, (u, vk), (u + 1.0, vk), t, dark);
            }
            for k in 0..3 {
                let vk = v + k as f32 / 3.0 + 0.08;
                cell.line(s, (u, vk), (u + 1.0, vk), t, light);
            }
            // Log ends where the run stops, as rings.
            for (open, ue) in [(lo_end, u + 0.14), (hi_end, u + 0.86)] {
                if !open {
                    continue;
                }
                for k in 0..3 {
                    let vc = v + (k as f32 + 0.5) / 3.0;
                    cell.block(s, (ue, vc), (0.16, 0.2), shade(c, 1.15, fade), dark, t);
                }
            }
        }
        Pattern::Stipple | Pattern::Earth => {
            let (count, alpha) = if pat == Pattern::Stipple { (16, 1.0) } else { (8, 0.6) };
            for i in 0..count {
                let (du, dv) = (h(x, y, 10 + i), h(x, y, 40 + i));
                let r = ((0.012 + 0.012 * h(x, y, 70 + i)) * z).max(0.6);
                let col =
                    if i % 2 == 0 { shade(c, 0.6, 0.85 * fade * alpha) } else { shade(c, 1.3, 0.6 * fade * alpha) };
                cell.dot(s, (u + du, v + dv), r, col);
            }
            if pat == Pattern::Stipple {
                // A lift line: cob goes up in lifts.
                let vl = v + 0.5 + (h(x, y, 3) - 0.5) * 0.08;
                cell.line(s, (u, vl), (u + 1.0, vl), t, shade(c, 0.7, 0.45 * fade));
            }
        }
        Pattern::Rubble | Pattern::Cobbles => {
            let n = if pat == Pattern::Rubble { 3 } else { 2 };
            for i in 0..n {
                for j in 0..n {
                    let (gu, gv) = (u as i32 * n + i, v as i32 * n + j);
                    let cu = (gu as f32 + 0.5) / n as f32 + (h(gu, gv, 1) - 0.5) * 0.06;
                    let cv = (gv as f32 + 0.5) / n as f32 + (h(gu, gv, 2) - 0.5) * 0.06;
                    let size = 0.75 / n as f32;
                    let (su, sv) = (size + h(gu, gv, 3) * 0.08, size * 0.9 + h(gu, gv, 4) * 0.08);
                    let fill = shade(c, 0.9 + h(gu, gv, 5) * 0.25, fade);
                    cell.block(s, (cu, cv), (su, sv), fill, dark, t);
                }
            }
        }
        Pattern::Weave => {
            // Two withies weaving past stakes every half cell.
            cell.line(s, (u, v + 0.17), (u + 1.0, v + 0.17), t, dark);
            cell.line(s, (u, v + 0.83), (u + 1.0, v + 0.83), t, dark);
            for side in [1.0f32, -1.0] {
                let col = if side > 0.0 { shade(c, 1.25, 0.8 * fade) } else { shade(c, 0.72, 0.9 * fade) };
                for k in 0..2 {
                    let ua = u + k as f32 * 0.5;
                    let sg = if (ua * 2.0).round() as i32 % 2 == 0 { side } else { -side };
                    const STEPS: usize = 4;
                    for q in 0..STEPS {
                        let at = |q: usize| {
                            let f = q as f32 / STEPS as f32;
                            (ua + 0.5 * f, v + 0.5 + sg * 0.17 * (std::f32::consts::PI * f).sin())
                        };
                        cell.line(s, at(q), at(q + 1), t * 1.7, col);
                    }
                }
            }
            for k in 0..2 {
                cell.dot(s, (u + k as f32 * 0.5, v + 0.5), (z * 0.05).max(1.0), shade(c, 0.45, fade));
            }
        }
        Pattern::Rushes => {
            for k in 0..5 {
                let o = h(x, y, k) * 0.2;
                let uk = u + k as f32 * 0.2 + o;
                cell.line(s, (uk, v + 0.08), (uk + 0.1, v + 0.92), t, shade(c, 0.7, 0.5 * fade));
            }
        }
        Pattern::Crag => {
            for i in 0..2 {
                let (mut cu, mut cv) = (u + 0.15 + h(x, y, 20 + i) * 0.7, v + 0.15 + h(x, y, 30 + i) * 0.7);
                for k in 0..3 {
                    let a = h(x, y, 50 + i * 3 + k) * std::f32::consts::TAU;
                    let (nu, nv) = (cu + a.cos() * 0.16, cv + a.sin() * 0.16);
                    cell.line(s, (cu, cv), (nu, nv), t, shade(c, 0.55, 0.7 * fade));
                    (cu, cv) = (nu, nv);
                }
            }
        }
    }
}
