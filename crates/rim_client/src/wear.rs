//! How far along a worked thing looks (DESIGN.md §6b): a plan rising
//! through its layers' `grow` windows, a rock cracking from the side it is
//! mined from, a tree leaning away from the axe. Everything is drawn from
//! `World::stage` and `Work`, so a cached thing is right until the sim
//! touches its cell, and a live one moves smoothly.

use crate::draw::Sink;
use macroquad::prelude::Color;
use rim_sim::defs::{DefId, HarvestDef, Targets, ThingDef, WorkStyleDef};
use rim_sim::hecs::Entity;
use rim_sim::look::{Layer, Prim};
use rim_sim::rng::hash2_f;
use rim_sim::world::{Side, Thing, Work, World};
use rim_sim::IVec;

/// How far along `e` is, 0 to 1. A worksite is drawn every frame, so it gets
/// the exact figure; anything else is cached and shows its stage, which is
/// all the cache hears about.
pub fn progress(w: &World, e: Entity) -> f32 {
    if !w.is_worksite(e) {
        return w.stage(e) as f32 / 8.0;
    }
    let worked = w.ecs.get::<&Work>(e).map_or(0.0, |k| k.done as f32 / k.total.max(1) as f32);
    let hurt = match (w.ecs.get::<&Thing>(e), w.stat(e, "hp")) {
        (Ok(t), Some(max)) if max >= 1.0 => (1.0 - t.hp as f32 / max as f32).clamp(0.0, 1.0),
        _ => 0.0,
    };
    worked.max(hurt)
}

/// The style `e`'s wear is drawn in: the one its work is for, or, when it
/// is only damaged, the one that would take it down.
pub fn style<'a>(w: &'a World, e: Entity, t: &Thing) -> Option<&'a WorkStyleDef> {
    style_id(w, e, t).map(|s| &w.defs.work_styles[s as usize])
}

pub fn style_id(w: &World, e: Entity, t: &Thing) -> Option<DefId> {
    let defs = &w.defs;
    let td = defs.thing(t.def);
    let desig = match w.ecs.get::<&Work>(e).ok().and_then(|k| k.designation) {
        Some(d) => d,
        None if td.natural => felled_by(td)?.desig_r,
        None => defs.designations.iter().position(|d| d.targets == Targets::Built)? as DefId,
    };
    defs.designations[desig as usize].style_r
}

/// The harvest that takes a natural thing away (a chop, a mine), else its
/// first: the one damage looks like.
pub fn felled_by(td: &ThingDef) -> Option<&HarvestDef> {
    td.harvest.iter().find(|h| h.destroy).or(td.harvest.first())
}

/// The side the last blow came from, as a step from the thing toward it.
pub fn toward(w: &World, e: Entity) -> (f32, f32) {
    match w.ecs.get::<&Work>(e).map_or(Side::default(), |k| k.side) {
        Side::North => (0.0, -1.0),
        Side::East => (1.0, 0.0),
        Side::South => (0.0, 1.0),
        Side::West => (-1.0, 0.0),
    }
}

/// `layers` as they look `f` of the way through being built: each inside
/// its window, the ones not started left out, all at `alpha`. Returns the
/// layers and the top of what stands, in cells from the cell's top.
pub fn grown(layers: &[Layer], f: f32, own: Color, alpha: f32) -> (Vec<Layer>, f32) {
    let n = layers.len();
    let mut out = Vec::with_capacity(n);
    let mut top: f32 = 1.0;
    for (i, l) in layers.iter().enumerate() {
        let [from, to] = l.window(i, n);
        let g = ((f - from) / (to - from)).clamp(0.0, 1.0);
        if g <= 0.0 {
            continue;
        }
        let mut l = *l;
        let mut fade = alpha;
        match &mut l.prim {
            Prim::Fill { rect, .. } | Prim::Outline { rect, .. } => {
                let h = rect[3] * g;
                rect[1] += rect[3] - h;
                rect[3] = h;
                top = top.min(rect[1]);
            }
            Prim::Disc { r, min_px, .. } => {
                *r *= g;
                *min_px *= g;
            }
            // Clipping a picture would squash it: it fades in instead.
            Prim::Edges { .. } | Prim::Sprite { .. } | Prim::Glyph { .. } => fade *= g,
        }
        let base = l.color.map_or(own, |[r, g, b, a]| Color::from_rgba(r, g, b, a));
        l.color = Some(Color::new(base.r, base.g, base.b, base.a * fade).into());
        out.push(l);
    }
    (out, top)
}

/// Blueprint hatching over `[x, y, w, h]` in points: 45° strokes, clipped.
pub fn hatch(s: &mut impl Sink, (x, y, w, h): (f32, f32, f32, f32), step: f32, c: Color) {
    let mut k = -h + step / 2.0;
    while k < w {
        // The stroke starts on the bottom or left edge and runs up and
        // right until it leaves the rectangle.
        let (dx, dy) = (k.max(0.0), (-k).max(0.0));
        let len = (w - dx).min(h - dy);
        if len > 0.0 {
            s.line(x + dx, y + h - dy, x + dx + len, y + h - dy - len, 1.0, c);
        }
        k += step;
    }
}

/// Crack segments, three branches from the middle of the worked side. Each
/// branch grows a step per round, so revealing them in order spreads the
/// cracks outward. The same cell always cracks the same way.
const BRANCHES: usize = 3;
const STEPS: usize = 6;
pub const CRACKS: usize = BRANCHES * STEPS;

fn cracks(cell: IVec) -> [[f32; 4]; CRACKS] {
    let mut out = [[0.0; 4]; CRACKS];
    let r = |i: usize, k: u64| hash2_f(cell.x as i64 * 31 + i as i64, cell.y as i64, 50 + k) as f32 - 0.5;
    let mut at = [[0.02, 0.5]; BRANCHES];
    let mut heading = [-0.7f32, 0.0, 0.7];
    for step in 0..STEPS {
        for b in 0..BRANCHES {
            heading[b] += r(step * BRANCHES + b, 1) * 0.9;
            let [x, y] = at[b];
            let nx = (x + heading[b].cos() * 0.13).clamp(0.04, 0.96);
            let ny = (y + heading[b].sin() * 0.13).clamp(0.04, 0.96);
            out[step * BRANCHES + b] = [x, y, nx, ny];
            at[b] = [nx, ny];
        }
    }
    out
}

/// A point in the cracks' frame (the worked side is x = 0) turned to face
/// the worker at `toward`.
fn turn((u, v): (f32, f32), toward: (f32, f32)) -> (f32, f32) {
    match toward {
        (x, _) if x > 0.0 => (1.0 - u, v),
        (_, y) if y < 0.0 => (v, u),
        (_, y) if y > 0.0 => (v, 1.0 - u),
        _ => (u, v),
    }
}

/// `f` of the cracks across the cell whose top-left is at `(sx, sy)`.
pub fn draw_cracks(s: &mut impl Sink, cell: IVec, toward: (f32, f32), f: f32, (sx, sy): (f32, f32), z: f32) {
    let n = (f * CRACKS as f32).round() as usize;
    let c = Color::new(0.07, 0.06, 0.05, 0.75);
    let t = (z / 32.0).max(1.0);
    for [x0, y0, x1, y1] in cracks(cell).into_iter().take(n) {
        let (a, b) = (turn((x0, y0), toward), turn((x1, y1), toward));
        s.line(sx + a.0 * z, sy + a.1 * z, sx + b.0 * z, sy + b.1 * z, t, c);
    }
}

/// What has come off so far, lying on the worker's side: two chips a stage.
pub fn draw_chips(s: &mut impl Sink, cell: IVec, toward: (f32, f32), f: f32, (sx, sy): (f32, f32), z: f32, c: Color) {
    let n = (f * 8.0).floor() as i64 * 2;
    let size = (0.05 * z).max(1.5);
    for i in 0..n {
        let along = 0.15 + hash2_f(cell.x as i64, cell.y as i64 * 7 + i, 61) as f32 * 0.7 - 0.5;
        let out = 0.5 + hash2_f(cell.x as i64, cell.y as i64 * 7 + i, 62) as f32 * 0.45;
        let (x, y) = (0.5 + toward.0 * out + toward.1.abs() * along, 0.5 + toward.1 * out + toward.0.abs() * along);
        let k = 0.75 + hash2_f(cell.x as i64, i, 63) as f32 * 0.4;
        let c = Color::new((c.r * k).min(1.0), (c.g * k).min(1.0), (c.b * k).min(1.0), 0.9);
        s.rect(sx + x * z - size / 2.0, sy + y * z - size / 2.0, size, size, c);
    }
}
