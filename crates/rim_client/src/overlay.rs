//! World overlays (DESIGN.md §6f): what is picked, drawn in one language.
//!
//! `scene` turns the app's state into a list of marks in screen points,
//! and `draw` paints them over the lit world. The scene is plain data so
//! the autotest can check what the player would see without reading
//! pixels. Colours and sizes are theme tokens, so a mod restyles the
//! overlays the way it restyles a panel.

use crate::{draw, App};
use macroquad::prelude::*;
use rim_sim::hecs::Entity;
use rim_sim::world::Pawn;
use rim_ui::paint::Draw;
use rim_ui::theme::{Rgba, Theme};

/// Brackets and rings close in over this long, from `SELECT_FROM` points
/// further out than they settle.
const SELECT_IN_SECS: f64 = 0.12;
const SELECT_FROM: f32 = 4.0;
/// Everything in a group but the one the inspector shows.
const GROUP_ALPHA: f32 = 0.7;
/// A path's dots, this many points apart.
const DOT_STEP: f32 = 5.0;

/// Overlay colours and sizes, read from the theme. Sizes are points: the
/// world is drawn in points, and the UI scale is for panels, not the map.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub chalk: Color,
    pub keyline: Color,
    pub firm: f32,
    pub bracket_gap: f32,
    pub bracket_arm_min: f32,
    pub bracket_arm_max: f32,
    /// A chip is a small panel: the panels' ground, edge, text, corner
    /// and caption size.
    pub surface: Color,
    pub line: Color,
    pub text: Color,
    pub radius: f32,
    pub caption: f32,
    pub leading: f32,
}

impl Palette {
    /// The theme's tokens, falling back to core's values for any a theme
    /// leaves out, so a mod that replaces the theme still gets overlays.
    pub fn from_theme(t: &Theme) -> Palette {
        let c = |k: &str, hex: &str| {
            let v: Rgba = t.color.get(k).copied().or_else(|| rim_ui::theme::parse_color(hex)).unwrap_or([1.0; 4]);
            Color::new(v[0], v[1], v[2], v[3])
        };
        let shape = |k: &str, v: f32| t.shape.get(k).copied().unwrap_or(v);
        Palette {
            chalk: c("chalk", "#f2eee3"),
            keyline: c("keyline", "#080a0c8c"),
            firm: shape("firm", 2.0),
            bracket_gap: shape("bracket_gap", 3.0),
            bracket_arm_min: shape("bracket_arm_min", 4.0),
            bracket_arm_max: shape("bracket_arm_max", 12.0),
            surface: c("surface_raised", "#1c1f24f2"),
            line: c("line_strong", "#ffffff40"),
            text: c("text", "#e7e7e4"),
            radius: shape("radius", 4.0),
            caption: t.text.get("caption").copied().unwrap_or(11.0),
            leading: t.leading(false),
        }
    }

    /// A bracket's arm: 28% of the footprint's short side, clamped.
    pub fn bracket_arm(&self, w: f32, h: f32) -> f32 {
        (w.min(h) * 0.28).clamp(self.bracket_arm_min, self.bracket_arm_max)
    }
}

/// One thing the overlay draws, in screen points.
#[derive(Clone, Debug, PartialEq)]
pub enum Mark {
    /// Selection on a thing: corner brackets `gap` outside its footprint.
    Brackets {
        rect: [f32; 4],
        gap: f32,
        alpha: f32,
    },
    /// Selection on a pawn: a ring `gap` outside its body.
    Ring {
        center: (f32, f32),
        r: f32,
        gap: f32,
        alpha: f32,
    },
    /// What's left of a selected pawn's path, dotted.
    Path {
        points: Vec<(f32, f32)>,
        alpha: f32,
    },
    /// A selected stack's way to where it will be stored, dashed.
    Haul {
        from: (f32, f32),
        to: (f32, f32),
    },
    Chip(Chip),
}

/// A small label by the pointer or a mark: "3 selected", "Chop · 4 trees".
#[derive(Clone, Debug, PartialEq)]
pub struct Chip {
    /// Its top-left corner; it's kept on screen when drawn.
    pub at: (f32, f32),
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub marks: Vec<Mark>,
}

/// What the overlays remember between frames.
#[derive(Default)]
pub struct State {
    /// The frame's clock, for anything that moves.
    now: f64,
    /// When each selected thing was selected, to close its brackets in.
    since: Vec<(Entity, f64)>,
}

impl State {
    /// Keep `since` in step with the selection.
    pub fn update(&mut self, selected: &[Entity], now: f64) {
        self.now = now;
        self.since.retain(|(e, _)| selected.contains(e));
        for &e in selected {
            if !self.since.iter().any(|(s, _)| *s == e) {
                self.since.push((e, now));
            }
        }
    }

    fn since(&self, e: Entity) -> Option<f64> {
        self.since.iter().find(|(s, _)| *s == e).map(|&(_, t)| t)
    }
}

/// How far out a selection mark sits, `t` seconds after it was made:
/// from `SELECT_FROM` points beyond `gap` in to `gap`, easing out.
fn closing(gap: f32, t: f64) -> f32 {
    let k = (t / SELECT_IN_SECS).clamp(0.0, 1.0) as f32;
    let ease = 1.0 - (1.0 - k).powi(3);
    gap + SELECT_FROM * (1.0 - ease)
}

/// Does a screen rectangle touch the screen?
fn on_screen([x, y, w, h]: [f32; 4]) -> bool {
    x + w >= 0.0 && y + h >= 0.0 && x <= screen_width() && y <= screen_height()
}

/// The overlay for this frame.
pub fn scene(app: &App) -> Scene {
    let p = &app.palette;
    let w = &app.sim.world;
    let cam = &app.cam;
    let z = cam.zoom;
    let now = app.chalk.now;
    let mut marks = Vec::new();
    let picked = crate::selection(app);
    let group = picked.len() > 1;
    let gap_for = |e: Entity| app.chalk.since(e).map_or(p.bracket_gap, |t0| closing(p.bracket_gap, now - t0));
    // The inspector's one, when it's on screen: where a group's chip goes.
    let mut primary = None;
    for &e in &picked {
        let alpha = if !group || app.selected == Some(e) { 1.0 } else { GROUP_ALPHA };
        let gap = gap_for(e);
        if let Some((center, r)) = draw::pawn_disc(app, e) {
            if let Ok(pawn) = w.ecs.get::<&Pawn>(e) {
                let mut points = vec![center];
                points.extend(pawn.path.iter().rev().map(|s| cam.to_screen(s.x as f32 + 0.5, s.y as f32 + 0.5)));
                if points.len() > 1 {
                    marks.push(Mark::Path { points, alpha });
                }
            }
            let out = r + gap;
            if on_screen([center.0 - out, center.1 - out, out * 2.0, out * 2.0]) {
                marks.push(Mark::Ring { center, r, gap, alpha });
                if alpha == 1.0 {
                    primary = Some((center.0 + out + 6.0, center.1 - 10.0));
                }
            }
        } else if let Some(t) = w.thing(e) {
            let [fw, fh] = w.defs.thing(t.def).size_facing(t.facing);
            let (sx, sy) = cam.to_screen(t.pos.x as f32, t.pos.y as f32);
            let rect = [sx, sy, z * fw as f32, z * fh as f32];
            if on_screen([rect[0] - gap, rect[1] - gap, rect[2] + 2.0 * gap, rect[3] + 2.0 * gap]) {
                marks.push(Mark::Brackets { rect, gap, alpha });
                if alpha == 1.0 {
                    primary = Some((rect[0] + rect[2] + gap + 6.0, rect[1] - gap));
                }
            }
        }
    }
    // Beside the inspector's one: speech sits above a pawn and its name
    // below, so a chip there would be covered.
    if let (true, Some(at)) = (group, primary) {
        marks.push(Mark::Chip(Chip { at, text: format!("{} selected", picked.len()) }));
    }
    // With the storage overlay on, a selected loose stack shows where it
    // will be carried.
    if app.storage_overlay {
        if let Some(e) = app.selected {
            if let (Some(t), Some(rim_sim::ai::HaulPlan::Moves { to, .. })) = (w.thing(e), rim_sim::ai::haul_plan(w, e))
            {
                let from = cam.to_screen(t.pos.x as f32 + 0.5, t.pos.y as f32 + 0.5);
                marks.push(Mark::Haul { from, to: cam.to_screen(to.x as f32 + 0.5, to.y as f32 + 0.5) });
            }
        }
    }
    Scene { marks }
}

fn fade(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * a)
}

/// Four corner brackets around `rect`, `gap` outside it, each on a
/// keyline. An arm is a pair of rectangles, so corners are square at any
/// width; each corner's keyline is two rectangles that don't overlap, so
/// its translucency doesn't double at the corner.
pub fn brackets(p: &Palette, [x, y, w, h]: [f32; 4], gap: f32, c: Color, a: f32) {
    let (x0, y0, x1, y1) = (x - gap, y - gap, x + w + gap, y + h + gap);
    let (arm, t) = (p.bracket_arm(w, h), p.firm);
    let h2 = t / 2.0;
    let key = fade(p.keyline, a);
    for (cx, cy, sx, sy) in [(x0, y0, 1.0, 1.0), (x1, y0, -1.0, 1.0), (x0, y1, 1.0, -1.0), (x1, y1, -1.0, -1.0)] {
        // The corner's outer edge, and how far each arm runs in from it.
        let (ox, oy) = (cx - sx * h2, cy - sy * h2);
        // `k` is the keyline's width beyond the stroke.
        let (len, k) = (arm + h2, 1.0);
        let span = |o: f32, s: f32, n: f32| if s > 0.0 { (o, n) } else { (o - n, n) };
        // Across: the arm along x, the full thickness, keyline on both sides.
        let (ax, aw) = span(ox - sx * k, sx, len + k * 2.0);
        let (ay, ah) = span(oy - sy * k, sy, t + k * 2.0);
        draw_rectangle(ax, ay, aw, ah, key);
        // Down: the arm along y, starting where the across keyline ends.
        let (bx, bw) = span(ox - sx * k, sx, t + k * 2.0);
        let (by, bh) = span(oy + sy * (t + k), sy, len - t);
        draw_rectangle(bx, by, bw, bh, key);
        let (hx, hw) = span(ox, sx, len);
        let (hy, hh) = span(oy, sy, t);
        draw_rectangle(hx, hy, hw, hh, fade(c, a));
        let (vx, vw) = span(ox, sx, t);
        let (vy, vh) = span(oy + sy * t, sy, len - t);
        draw_rectangle(vx, vy, vw, vh, fade(c, a));
    }
}

/// A ring with a keyline.
pub fn ring(p: &Palette, (x, y): (f32, f32), r: f32, t: f32, c: Color, a: f32) {
    draw_circle_lines(x, y, r, t + 2.0, fade(p.keyline, a));
    draw_circle_lines(x, y, r, t, fade(c, a));
}

/// Dots every few points along a polyline, each on a keyline. Only the
/// dots on screen are drawn: a long walk is mostly elsewhere.
fn dotted(p: &Palette, points: &[(f32, f32)], c: Color, a: f32) {
    let (sw, sh) = (screen_width(), screen_height());
    let mut carry = 0.0;
    for pair in points.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        let len = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let seen = on_screen([x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs()]);
        let mut d = carry;
        while d < len {
            if seen {
                let f = d / len;
                let (x, y) = (x0 + (x1 - x0) * f, y0 + (y1 - y0) * f);
                if (0.0..=sw).contains(&x) && (0.0..=sh).contains(&y) {
                    draw_circle(x, y, 1.6, fade(p.keyline, a));
                    draw_circle(x, y, 0.9, fade(c, a));
                }
            }
            d += DOT_STEP;
        }
        carry = d - len;
    }
}

/// Dashes a third of a cell long from `a` to `b`, on a keyline.
fn dashed(p: &Palette, (ax, ay): (f32, f32), (bx, by): (f32, f32), z: f32) {
    let len = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt().max(1.0);
    let dash = (z * 0.3).max(4.0);
    let mut d = 0.0;
    while d < len {
        let (f, g) = (d / len, (d + dash).min(len) / len);
        let (x0, y0, x1, y1) = (ax + (bx - ax) * f, ay + (by - ay) * f, ax + (bx - ax) * g, ay + (by - ay) * g);
        draw_line(x0, y0, x1, y1, p.firm + 2.0, p.keyline);
        draw_line(x0, y0, x1, y1, p.firm, p.chalk);
        d += dash * 2.0;
    }
}

/// Paint the scene's world marks. Chips are text: `chips` turns them into
/// the UI's draw list.
pub fn draw(scene: &Scene, p: &Palette, zoom: f32) {
    for m in &scene.marks {
        match m {
            Mark::Path { points, alpha } => dotted(p, points, p.chalk, 0.6 * alpha),
            Mark::Haul { from, to } => dashed(p, *from, *to, zoom),
            Mark::Brackets { rect, gap, alpha } => brackets(p, *rect, *gap, p.chalk, *alpha),
            // A point further out than brackets: a ring's line is round,
            // and hover sits on the body's edge inside it.
            Mark::Ring { center, r, gap, alpha } => ring(p, *center, r + gap + 1.0, p.firm, p.chalk, *alpha),
            Mark::Chip(_) => {}
        }
    }
}

/// The scene's chips as UI draws, in physical pixels: a small panel with
/// the text in the UI's font, kept on screen.
pub fn chips(scene: &Scene, p: &Palette, text: &mut rim_ui::text::Text, dpi: f32) -> Vec<Draw> {
    let rgba = |c: Color| [c.r, c.g, c.b, c.a];
    let (sw, sh) = (screen_width(), screen_height());
    let (pad, size) = (p.caption * 0.64, p.caption);
    let h = (size * p.leading + pad).round();
    let mut out = Vec::new();
    for m in &scene.marks {
        let Mark::Chip(c) = m else { continue };
        let mut quads = text.quads(&c.text, size * dpi, 500, 0.0, None, 0.0, 0.0);
        let tw = quads.iter().map(|q| (q.dst[0] + q.dst[2]) / dpi).fold(0.0f32, f32::max);
        let w = (tw + pad * 2.0).ceil();
        let x = c.at.0.clamp(4.0, (sw - w - 4.0).max(4.0));
        let y = c.at.1.clamp(4.0, (sh - h - 4.0).max(4.0));
        let r = [x * dpi, y * dpi, w * dpi, h * dpi];
        out.push(Draw::Rect { rect: r, color: rgba(p.surface), radius: p.radius * dpi });
        out.push(Draw::Outline { rect: r, color: rgba(p.line), width: dpi, radius: p.radius * dpi });
        // Quads are laid out from the line's top; centre the line in the chip.
        let (dx, dy) = (((x + pad) * dpi).round(), ((y + (h - size * p.leading) / 2.0) * dpi).round());
        for q in &mut quads {
            q.dst[0] += dx;
            q.dst[1] += dy;
        }
        out.push(Draw::Glyphs { quads, color: rgba(p.text) });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_theme_that_sets_chalk_changes_the_selection_colour() {
        let mut t = Theme::default();
        let core = Palette::from_theme(&t);
        assert_eq!(core.chalk, Color::new(0xf2 as f32 / 255.0, 0xee as f32 / 255.0, 0xe3 as f32 / 255.0, 1.0));
        t.color.insert("chalk".into(), [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(Palette::from_theme(&t).chalk, Color::new(1.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn bracket_arms_follow_the_footprint_within_limits() {
        let p = Palette::from_theme(&Theme::default());
        assert_eq!(p.bracket_arm(10.0, 10.0), 4.0);
        assert_eq!(p.bracket_arm(100.0, 100.0), 12.0);
        assert!((p.bracket_arm(28.0, 56.0) - 7.84).abs() < 1e-4);
    }

    #[test]
    fn selection_closes_in_to_its_gap_whatever_the_gap() {
        for gap in [3.0, 10.0] {
            assert_eq!(closing(gap, 0.0), gap + SELECT_FROM);
            assert_eq!(closing(gap, SELECT_IN_SECS), gap);
            assert_eq!(closing(gap, 5.0), gap);
            let mid = closing(gap, SELECT_IN_SECS / 2.0);
            assert!(mid > gap && mid < gap + SELECT_FROM);
        }
    }
}
