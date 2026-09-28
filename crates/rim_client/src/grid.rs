//! The grid (DESIGN.md §6f): it shows while a tool is in hand.
//!
//! A line is a groove in the ground, not a line over it: a dark seam on
//! the cell boundary with a faint lit edge beside it, from the same
//! top-left light as the walls. It's drawn after the terrain and before
//! the things, so whatever stands on a cell hides it.

use crate::overlay::fade;
use crate::{draw, App, Tool};
use macroquad::prelude::*;

/// How far the lens reaches from the pointer, in cells.
pub const LENS_CELLS: f32 = 5.5;
/// Below this many points a cell there's no grid at all; below
/// `GRID_NEAR_ZOOM` it's at 70%.
pub const GRID_MID_ZOOM: f32 = 10.0;
pub const GRID_NEAR_ZOOM: f32 = 20.0;
/// Fades: up quickly, down slowly and only after a hold, so swapping one
/// tool for another doesn't blink the grid.
const UP_SECS: f64 = 0.16;
const DOWN_SECS: f64 = 0.28;
const HOLD_SECS: f64 = 0.4;
/// The lit edge's strength against chalk, and the lens's extra seam:
/// half as much again as a line's own.
const LIT: f32 = 0.045;
const LENS_BOOST: f32 = 1.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// Nothing to place: no grid.
    Rest,
    /// A tool is armed: ticks at the corners around the pointer.
    Lens,
    /// A drag is under way: every line, strongest near the pointer.
    Plan,
}

/// The grid a frame wants. A select drag snaps to cells, so it gets the
/// lens too.
pub fn level(tool: Tool, dragging: bool) -> Level {
    match (tool, dragging) {
        (Tool::Select, false) => Level::Rest,
        (Tool::Select, true) | (_, false) => Level::Lens,
        (_, true) => Level::Plan,
    }
}

/// One level's strength, 0 to 1, fading toward what's wanted.
#[derive(Clone, Copy, Debug, Default)]
struct Fade {
    shown: f32,
    /// When it was last wanted, for the hold before fading out.
    wanted_at: f64,
}

impl Fade {
    /// `hold` keeps it up a moment after it stops being wanted: for the
    /// grid going away, not for one level handing over to another.
    fn step(&mut self, wanted: bool, hold: bool, now: f64, dt: f64) {
        if wanted {
            self.wanted_at = now;
            self.shown = (self.shown + (dt / UP_SECS) as f32).min(1.0);
        } else if !hold || now - self.wanted_at >= HOLD_SECS {
            self.shown = (self.shown - (dt / DOWN_SECS) as f32).max(0.0);
        }
    }
}

/// The grid between frames: each level's fade, and where the pointer is.
#[derive(Default)]
pub struct Grid {
    lens: Fade,
    plan: Fade,
    last: f64,
    /// The pointer in world tiles, when it's over the map.
    pointer: Option<(f32, f32)>,
}

impl Grid {
    pub fn update(&mut self, level: Level, pointer: Option<(f32, f32)>, now: f64) {
        let dt = (now - self.last).clamp(0.0, 0.1);
        self.last = now;
        let hold = level == Level::Rest;
        self.lens.step(level == Level::Lens, hold, now, dt);
        self.plan.step(level == Level::Plan, hold, now, dt);
        self.pointer = pointer;
    }
}

/// How much of the lens reaches `(x, y)`, in tiles: 1 at the pointer, 0
/// at `LENS_CELLS`, smoothly.
fn lens_at(pointer: (f32, f32), x: f32, y: f32) -> f32 {
    let d = ((x - pointer.0).powi(2) + (y - pointer.1).powi(2)).sqrt() / LENS_CELLS;
    let t = (1.0 - d).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How strong the grid is at this zoom.
fn band(zoom: f32) -> f32 {
    if zoom < GRID_MID_ZOOM {
        0.0
    } else if zoom < GRID_NEAR_ZOOM {
        0.7
    } else {
        1.0
    }
}

/// How strongly the ticks and the lines draw this frame, 0 to 1: their
/// fades, at this zoom.
pub fn strength(app: &App) -> (f32, f32) {
    let k = band(app.cam.zoom);
    (app.grid.lens.shown * k, app.grid.plan.shown * k)
}

/// Draw the grid over the ground. Call it before the things.
pub fn draw(app: &App) {
    let g = &app.grid;
    let (cam, p) = (&app.cam, &app.palette);
    let (lens, plan) = strength(app);
    if lens <= 0.0 && plan <= 0.0 {
        return;
    }
    let map = &app.sim.world.map;
    let (x0, y0, x1, y1) = draw::visible(app);
    let (x1, y1) = ((x1 + 1).min(map.w), (y1 + 1).min(map.h));
    // A line is a point wide, rounded to whole pixels of the world's
    // target and never less than one, with positions on those pixels: it
    // doesn't shimmer at a render scale below 1.
    let dot = 1.0 / (screen_dpi_scale() * app.render_scale.unwrap_or(1.0));
    let px = (1.0 / dot).round().max(1.0) * dot;
    let snap = |v: f32| (v / dot).round() * dot;
    let sx = |gx: i32| snap(cam.to_screen(gx as f32, 0.0).0);
    let sy = |gy: i32| snap(cam.to_screen(0.0, gy as f32).1);
    let (left, top, right, bottom) = (sx(x0), sy(y0), sx(x1), sy(y1));
    let seam = |x: f32, y: f32, w: f32, h: f32, a: f32| draw_rectangle(x, y, w, h, fade(p.seam, a));
    if plan > 0.0 {
        // A seam, and its lit edge a pixel to the lower right.
        for gx in x0..=x1 {
            seam(sx(gx), top, px, bottom - top, plan);
            draw_rectangle(sx(gx) + px, top, px, bottom - top, fade(p.chalk, LIT * plan));
        }
        for gy in y0..=y1 {
            seam(left, sy(gy), right - left, px, plan);
            draw_rectangle(left, sy(gy) + px, right - left, px, fade(p.chalk, LIT * plan));
        }
    }
    let Some(ptr) = g.pointer else { return };
    let reach = LENS_CELLS.ceil() as i32 + 1;
    let (cx, cy) = (ptr.0.floor() as i32, ptr.1.floor() as i32);
    let arm = (cam.zoom * 0.14).clamp(2.0, 4.0).round();
    for gy in (cy - reach).max(0)..=(cy + reach).min(map.h) {
        for gx in (cx - reach).max(0)..=(cx + reach).min(map.w) {
            let (x, y) = (sx(gx), sy(gy));
            if plan > 0.0 {
                // The lens deepens the seams near the pointer.
                let down = lens_at(ptr, gx as f32, gy as f32 + 0.5) * LENS_BOOST * plan;
                if gy < map.h && down > 0.01 {
                    seam(x, y, px, sy(gy + 1) - y, down);
                }
                let across = lens_at(ptr, gx as f32 + 0.5, gy as f32) * LENS_BOOST * plan;
                if gx < map.w && across > 0.01 {
                    seam(x, y, sx(gx + 1) - x, px, across);
                }
            }
            let a = lens_at(ptr, gx as f32, gy as f32) * lens;
            if a > 0.03 {
                // A plus at the corner: the seam dark, the chalk lit.
                let dark = fade(p.seam, 4.0 * a);
                draw_rectangle(x - arm, y, arm * 2.0 + px, px, dark);
                draw_rectangle(x, y - arm, px, arm * 2.0 + px, dark);
                let lit = fade(p.chalk, 0.45 * a);
                draw_rectangle(x - arm + px, y + px, arm * 2.0 + px, px, lit);
                draw_rectangle(x + px, y - arm + px, px, arm * 2.0 + px, lit);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grid_shows_while_a_tool_is_in_hand() {
        let build = Tool::Build(0);
        assert_eq!(level(Tool::Select, false), Level::Rest);
        assert_eq!(level(Tool::Select, true), Level::Lens);
        for tool in [build, Tool::Designate(0), Tool::Stockpile, Tool::ClearZone, Tool::Cancel] {
            assert_eq!(level(tool, false), Level::Lens, "{tool:?} armed");
            assert_eq!(level(tool, true), Level::Plan, "{tool:?} dragging");
        }
    }

    #[test]
    fn the_lens_fades_out_over_its_reach() {
        assert_eq!(lens_at((5.0, 5.0), 5.0, 5.0), 1.0);
        assert_eq!(lens_at((5.0, 5.0), 5.0 + LENS_CELLS, 5.0), 0.0);
        let half = lens_at((5.0, 5.0), 5.0 + LENS_CELLS / 2.0, 5.0);
        assert!((half - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_tool_swap_holds_the_grid_up() {
        let mut g = Grid::default();
        for i in 0..30 {
            g.update(Level::Lens, None, i as f64 / 60.0);
        }
        assert_eq!(g.lens.shown, 1.0);
        // Put down for less than the hold: still up.
        g.update(Level::Rest, None, 0.5 + 0.2);
        assert_eq!(g.lens.shown, 1.0);
        // Long after: gone.
        let mut t = 0.7;
        while t < 2.0 {
            t += 1.0 / 60.0;
            g.update(Level::Rest, None, t);
        }
        assert_eq!(g.lens.shown, 0.0);
    }

    #[test]
    fn starting_a_drag_hands_the_ticks_over_to_the_lines() {
        let mut g = Grid::default();
        for i in 0..30 {
            g.update(Level::Lens, None, i as f64 / 60.0);
        }
        let mut t = 0.5;
        for _ in 0..20 {
            t += 1.0 / 60.0;
            g.update(Level::Plan, None, t);
        }
        assert_eq!(g.lens.shown, 0.0);
        assert_eq!(g.plan.shown, 1.0);
    }

    #[test]
    fn no_grid_when_cells_are_small() {
        assert_eq!(band(8.0), 0.0);
        assert_eq!(band(14.0), 0.7);
        assert_eq!(band(28.0), 1.0);
    }
}
