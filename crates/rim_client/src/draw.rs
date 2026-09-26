//! Rendering. Read-only access to the simulation.

use crate::atlas::{Slot, WorldAtlas};
use crate::wear;
use crate::worksite::{Kind, Tone, DETAIL_ZOOM};
use crate::{rgb, App, Tool};
use macroquad::prelude::*;
use rim_sim::defs::Exit;
use rim_sim::defs::Wear;
use rim_sim::hecs::Entity;
use rim_sim::look::{Layer, Prim};
use rim_sim::map::CHUNK;
use rim_sim::rng::hash2_f;
use rim_sim::world::*;
use rim_sim::IVec;
use rim_ui::paint::Draw;

const PLAYER: Color = Color::new(0.35, 0.8, 1.0, 1.0);
const HOSTILE: Color = Color::new(1.0, 0.3, 0.25, 1.0);

/// Interpolated position of a pawn's center, in tiles.
pub fn pawn_pos(p: &Pawn) -> (f32, f32) {
    let (x, y) = (p.pos.x as f32 + 0.5, p.pos.y as f32 + 0.5);
    match p.next {
        Some(n) => {
            let t = p.progress as f32 / p.step_ticks.max(1) as f32;
            (x + (n.x as f32 + 0.5 - x) * t, y + (n.y as f32 + 0.5 - y) * t)
        }
        None => (x, y),
    }
}

fn shade(c: Color, f: f32) -> Color {
    Color::new((c.r * f).min(1.0), (c.g * f).min(1.0), (c.b * f).min(1.0), c.a)
}

fn alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

/// The terrain as a texture, one texel per cell with the per-tile variation
/// baked in, drawn as a single quad. Drawing a rectangle per cell cost ~100
/// batched draw calls zoomed out; this is one. A chunk is re-uploaded when
/// its terrain changes, and nothing else: chopping a tree doesn't touch it.
#[derive(Default)]
pub struct Ground {
    tex: Option<Texture2D>,
    /// The terrain revision each chunk was last uploaded at.
    revs: Vec<u64>,
}

impl Ground {
    /// RGBA for the cells in `[x0, x0 + w) × [y0, y0 + h)`.
    fn texels(w: &World, x0: i32, y0: i32, cw: i32, ch: i32) -> Image {
        let mut bytes = vec![0u8; (cw * ch * 4) as usize];
        for y in 0..ch {
            for x in 0..cw {
                let p = IVec::new(x0 + x, y0 + y);
                let base = w.defs.terrain[w.map.terrain[w.map.idx(p)] as usize].rgb;
                let v = 0.94 + hash2_f(p.x as i64, p.y as i64, 99) as f32 * 0.1;
                let o = ((y * cw + x) * 4) as usize;
                for (k, c) in base.iter().enumerate() {
                    bytes[o + k] = (*c as f32 * v).min(255.0) as u8;
                }
                bytes[o + 3] = 255;
            }
        }
        Image { bytes, width: cw as u16, height: ch as u16 }
    }

    pub fn update(&mut self, w: &World) {
        let (mw, mh) = (w.map.w, w.map.h);
        let (cx, cy) = w.map.chunks();
        let fits = self.tex.as_ref().is_some_and(|t| t.width() as i32 == mw && t.height() as i32 == mh);
        if !fits {
            let t = Texture2D::from_image(&Self::texels(w, 0, 0, mw, mh));
            // Crisp cell edges when zoomed in, like the rectangles were.
            t.set_filter(FilterMode::Nearest);
            self.tex = Some(t);
            self.revs = (0..(cx * cy) as usize).map(|c| w.map.terrain_rev(c)).collect();
            return;
        }
        let Some(tex) = &self.tex else { return };
        for (c, seen) in self.revs.iter_mut().enumerate() {
            let now = w.map.terrain_rev(c);
            if *seen == now {
                continue;
            }
            *seen = now;
            let IVec { x: x0, y: y0 } = w.map.chunk_origin(c);
            let (cw, ch) = (CHUNK.min(mw - x0), CHUNK.min(mh - y0));
            tex.update_part(&Self::texels(w, x0, y0, cw, ch), x0, y0, cw, ch);
        }
    }
}

/// Where painting goes: straight to macroquad's batch each frame, or into
/// a chunk's cached buffers (mesh.rs). Both draw the same pixels.
pub trait Sink {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color);
    fn poly(&mut self, x: f32, y: f32, sides: u8, r: f32, c: Color);
    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, t: f32, c: Color);
    fn tri(&mut self, p: [[f32; 2]; 3], c: Color);
    /// A world atlas slot (a sprite, a glyph) stretched over a rectangle.
    fn image(&mut self, x: f32, y: f32, w: f32, h: f32, slot: Slot, c: Color);
    fn atlas(&self) -> &WorldAtlas;
}

/// Macroquad's batch, this frame.
pub struct Immediate<'a>(pub &'a WorldAtlas);

impl Sink for Immediate<'_> {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        draw_rectangle(x, y, w, h, c);
    }
    fn poly(&mut self, x: f32, y: f32, sides: u8, r: f32, c: Color) {
        draw_poly(x, y, sides, r, 0.0, c);
    }
    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, t: f32, c: Color) {
        draw_line(x0, y0, x1, y1, t, c);
    }
    fn tri(&mut self, [a, b, c2]: [[f32; 2]; 3], c: Color) {
        draw_triangle(vec2(a[0], a[1]), vec2(b[0], b[1]), vec2(c2[0], c2[1]), c);
    }
    fn atlas(&self) -> &WorldAtlas {
        self.0
    }
    fn image(&mut self, x: f32, y: f32, w: f32, h: f32, slot: Slot, c: Color) {
        let tex = &self.0.pages[slot.page];
        let (tw, th) = (tex.width(), tex.height());
        let [u0, v0, u1, v1] = slot.uv;
        let source = Rect::new(u0 * tw, v0 * th, (u1 - u0) * tw, (v1 - v0) * th);
        draw_texture_ex(
            tex,
            x,
            y,
            c,
            DrawTextureParams { dest_size: Some(vec2(w, h)), source: Some(source), ..Default::default() },
        );
    }
}

/// A filled circle, cheaper when small: under 6 px across a 20-sided circle
/// looks the same as an 8-sided one, at 24 indices instead of 60.
fn disc(s: &mut impl Sink, x: f32, y: f32, r: f32, c: Color) {
    s.poly(x, y, if r < 6.0 { 8 } else { 20 }, r, c);
}

/// A rectangle's outline, `t / 2` thick inside it, as macroquad's
/// `draw_rectangle_lines` draws it, from four strips that don't overlap.
fn outline(s: &mut impl Sink, x: f32, y: f32, w: f32, h: f32, t: f32, c: Color) {
    let b = t / 2.0;
    s.rect(x, y, w, b, c);
    s.rect(x, y + h - b, w, b, c);
    s.rect(x, y + b, b, h - t, c);
    s.rect(x + w - b, y + b, b, h - t, c);
}

/// The part of the viewport that holds map cells, in tiles, inclusive.
fn visible(app: &App) -> (i32, i32, i32, i32) {
    let (w, cam) = (&app.sim.world, &app.cam);
    let (x0, y0) = cam.to_world(0.0, 0.0);
    let (x1, y1) = cam.to_world(screen_width(), screen_height());
    (
        (x0.floor() as i32).max(0),
        (y0.floor() as i32).max(0),
        (x1.ceil() as i32).min(w.map.w - 1),
        (y1.ceil() as i32).min(w.map.h - 1),
    )
}

/// Stack counts to label, in screen points: (top-left x, y, count). The
/// client draws them with the UI's own text so they share its atlas and
/// batch (macroquad's `draw_text` broke the world's batch twice per label).
pub type Counts = Vec<(f32, f32, u32)>;

/// Paint thing `e`, which stands in `cell`, with the cell's top-left at
/// `at` and `z` points a side. Returns its count if it has a stack to label.
///
/// `tone` moves a live worksite this frame (DESIGN.md §6b): the shake and
/// flash of a blow, the settle of a build that just stood. The chunk cache
/// draws everything with the default, which changes nothing.
#[allow(clippy::too_many_arguments)]
pub fn thing(
    s: &mut impl Sink,
    w: &World,
    e: Entity,
    cell: IVec,
    at: (f32, f32),
    z: f32,
    t: f32,
    tone: Tone,
) -> Option<u32> {
    let defs = &w.defs;
    let th = w.ecs.get::<&Thing>(e).ok()?;
    // A thing covering several cells is one entity in each of them, and is
    // drawn once, from its anchor, across them all.
    if th.pos != cell {
        return None;
    }
    let td = defs.thing(th.def);
    let span = td.size;
    // A thing built of something is drawn in that something's colour, so
    // marble arrives looking like marble with no change here. Anything
    // else keeps its def's colour.
    let c = match w.ecs.get::<&MadeOf>(e) {
        Ok(m) => rgb(defs.thing(m.0).rgb),
        Err(_) => rgb(td.rgb),
    };
    let c = shade(c, tone.bright);
    // Scaled about the middle of the bottom edge, where it stands.
    let zt = z * tone.scale;
    let (fw, fh) = (span[0] as f32, span[1] as f32);
    let at = (at.0 + tone.shift.0 * z - (zt - z) * fw / 2.0, at.1 + tone.shift.1 * z - (zt - z) * fh);
    let z = zt;
    let (sx, sy) = at;
    let look = &td.look_r;
    let orient = orient_of(w, cell, look, span);
    if let Ok(bp) = w.ecs.get::<&Blueprint>(e) {
        plan(s, w, e, &bp, &look.layers, c, cell, at, z, t, span, orient);
        return None;
    }
    let layers = match w.ecs.get::<&Regrow>(e) {
        Ok(_) if !look.regrowing.is_empty() => &look.regrowing,
        _ => &look.layers,
    };
    let f = wear::progress(w, e);
    let style = (f > 0.0).then(|| wear::style(w, e, &th)).flatten();
    match style.map(|st| st.wear) {
        Some(Wear::Lean) => {
            // Away from the axe, more as the cut deepens.
            let (tx, ty) = wear::toward(w, e);
            let k = 0.1 * f * f * z;
            wear::draw_chips(s, cell, (tx, ty), f, at, z, chip_color(w, e, td, c));
            // In the last eighth, where it will come down.
            if f >= 7.0 / 8.0 && style.is_some_and(|st| st.exit == rim_sim::defs::Exit::Fall) {
                let (fx, fy) = wear::fall_way(w, cell, (tx, ty));
                for (k, a) in [(1.0, 0.16), (2.0, 0.1)] {
                    s.rect(sx + fx * k * z, sy + fy * k * z, z, z, Color::new(1.0, 0.84, 0.47, a));
                }
            }
            paint(s, w, layers, join_of(look), orient, c, cell, (sx - tx * k, sy - ty * k), z, t, span);
        }
        Some(Wear::Cracks) => {
            let toward = wear::toward(w, e);
            if w.ecs.get::<&Work>(e).is_ok() {
                wear::draw_chips(s, cell, toward, f, at, z, chip_color(w, e, td, c));
            }
            paint(s, w, layers, join_of(look), orient, shade(c, 1.0 - 0.16 * f), cell, at, z, t, span);
            // Every cell cracks from its own seed, so a boulder isn't copies.
            for q in td.footprint(cell) {
                let at = (sx + (q.x - cell.x) as f32 * z, sy + (q.y - cell.y) as f32 * z);
                wear::draw_cracks(s, q, toward, f, at, z);
            }
        }
        // Taken down in reverse of how it went up. Only for things that
        // don't block: the loader refuses it for those that do.
        Some(Wear::Grow) => {
            let (grown, _) = wear::grown(layers, 1.0 - f, c, 1.0);
            paint(s, w, &grown, join_of(look), orient, c, cell, at, z, t, span);
        }
        Some(Wear::None) | None => paint(s, w, layers, join_of(look), orient, c, cell, at, z, t, span),
    }
    if let Ok(d) = w.ecs.get::<&Designated>(e) {
        let dc = rgb(defs.designations[d.0 as usize].rgb);
        // At the footprint's top-right corner.
        let dx = sx + z * (fw - 0.18);
        disc(s, dx, sy + z * 0.18, z * 0.13 + 1.0, BLACK);
        disc(s, dx, sy + z * 0.18, z * 0.13, dc);
    }
    (th.count > 1).then_some(th.count)
}

/// A plan: what stands of it so far, rising through its layers' `grow`
/// windows see-through and hatched, because a plan doesn't block and
/// mustn't look as if it does. Above that, a faint ghost of the rest.
#[allow(clippy::too_many_arguments)]
fn plan(
    s: &mut impl Sink,
    w: &World,
    e: Entity,
    bp: &Blueprint,
    layers: &[Layer],
    own: Color,
    cell: IVec,
    at: (f32, f32),
    z: f32,
    t: f32,
    span: [u32; 2],
    orient: Orient,
) {
    const BLUEPRINT: Color = Color::new(0.55, 0.8, 1.0, 0.8);
    let (sx, sy) = at;
    let (zx, zy) = (z * span[0] as f32, z * span[1] as f32);
    let f = wear::progress(w, e);
    let (grown, top) = if f > 0.0 { wear::grown(layers, f, own, 0.55) } else { (Vec::new(), 1.0) };
    s.rect(sx + 1.0, sy + 1.0, zx - 2.0, (top * zy - 1.0).max(0.0), Color::new(0.45, 0.7, 1.0, 0.3));
    if !grown.is_empty() {
        // A plan joins nothing until it stands.
        paint(s, w, &grown, None, orient, own, cell, at, z, t, span);
        let hatched = (sx, sy + top * zy, zx, (1.0 - top) * zy);
        wear::hatch(s, hatched, (z / 7.0).max(4.0), Color::new(0.55, 0.8, 1.0, 0.4));
    }
    outline(s, sx + 1.0, sy + 1.0, zx - 2.0, zy - 2.0, 1.5, BLUEPRINT);
    // The materials that have arrived, stacked in the corner until used.
    let need: u32 = bp.cost.iter().map(|c| c.1).sum();
    let have: u32 = bp.delivered.iter().sum();
    let pile = 0.32 * have as f32 / need.max(1) as f32 * (1.0 - f);
    if let (true, Some(&(m, _))) = (pile > 0.04, bp.cost.first()) {
        let mc = rgb(w.defs.thing(m).rgb);
        s.rect(sx + 0.08 * z, sy + (0.92 - pile) * z, pile * z, pile * z, mc);
    }
    let frac = if have < need { have as f32 / need.max(1) as f32 * 0.5 } else { 0.5 + 0.5 * f };
    s.rect(sx + 2.0, sy + zy - 4.0, (zx - 4.0) * frac, 2.5, Color::new(0.6, 0.9, 1.0, 0.9));
}

/// What comes off a thing as it is worked: what it yields, or what it was
/// built of.
fn chip_color(w: &World, e: Entity, td: &rim_sim::defs::ThingDef, own: Color) -> Color {
    let worked = w.ecs.get::<&Work>(e).ok().and_then(|k| k.designation).and_then(|d| td.harvest_for(d));
    match worked.or(wear::felled_by(td)).and_then(|h| h.yields_r.first()) {
        Some(&(y, _)) if w.ecs.get::<&MadeOf>(e).is_err() => rgb(w.defs.thing(y).rgb),
        _ => own,
    }
}

/// Stacks show their count once a cell is big enough to read one.
const LABEL_ZOOM: f32 = 22.0;

/// Rock nobody is working (DESIGN.md §6d): terrain, drawn as the thing it
/// stands up as, so the cell looks the same before and after it's marked.
#[allow(clippy::too_many_arguments)]
pub fn rock(s: &mut impl Sink, w: &World, def: rim_sim::defs::DefId, cell: IVec, at: (f32, f32), z: f32, t: f32) {
    let td = w.defs.thing(def);
    let orient = orient_of(w, cell, &td.look_r, td.size);
    paint(s, w, &td.look_r.layers, join_of(&td.look_r), orient, rgb(td.rgb), cell, at, z, t, td.size);
}

/// The ground, then floors, items and fixtures: cached per chunk where
/// they don't change (mesh.rs), live where they do.
pub fn things(app: &mut App) -> Counts {
    let w = &app.sim.world;
    let cam = &app.cam;
    let z = cam.zoom;
    clear_background(Color::from_rgba(12, 14, 16, 255));

    // Terrain: one quad from the baked ground texture.
    if let Some(tex) = &app.ground.tex {
        let (sx, sy) = cam.to_screen(0.0, 0.0);
        draw_texture_ex(
            tex,
            sx,
            sy,
            WHITE,
            DrawTextureParams { dest_size: Some(vec2(w.map.w as f32 * z, w.map.h as f32 * z)), ..Default::default() },
        );
    }

    let t = get_time() as f32;
    app.meshes.prepare(w, &app.world_atlas, cam, t);
    let (tx0, ty0, tx1, ty1) = visible(app);
    let on_screen = |c: IVec| (tx0..=tx1).contains(&c.x) && (ty0..=ty1).contains(&c.y);
    let detail = z >= DETAIL_ZOOM;
    let mut counts = Counts::new();
    let mut label = |cell: IVec, n: u32| {
        if z >= LABEL_ZOOM && on_screen(cell) {
            let (sx, sy) = cam.to_screen(cell.x as f32, cell.y as f32);
            counts.push((sx + z * 0.22, sy + z * 0.95 - 12.0, n));
        }
    };
    // Per layer, cached then live, so a plan never covers what stands on it.
    for layer in 0..3 {
        app.meshes.draw_layer(w, cam, layer, app.world_target.as_ref().map(|t| t.render_pass.raw_miniquad_id()));
        for cell in app.meshes.live(layer) {
            let Some(e) = w.map.layers_at(w.map.idx(cell))[layer] else { continue };
            let at = cam.to_screen(cell.x as f32, cell.y as f32);
            let tone = app.worksites.tone(w, e, detail);
            if let Some(n) = thing(&mut Immediate(&app.world_atlas), w, e, cell, at, z, t, tone) {
                label(cell, n);
            }
        }
    }
    for (cell, n) in app.meshes.counts() {
        label(cell, n);
    }
    worksite_motion(app, t);
    counts
}

/// Worksite motion over the things (DESIGN.md §6b): what is leaving, and
/// what the blows throw. Zoomed out, a blow is a brief flash of the cell's
/// outline instead, so a busy colony still reads as busy.
fn worksite_motion(app: &App, t: f32) {
    let (w, cam, ws) = (&app.sim.world, &app.cam, &app.worksites);
    let z = cam.zoom;
    let s = &mut Immediate(&app.world_atlas);
    for l in &ws.leaving {
        let p = ws.exit_age(l);
        if p >= 1.0 {
            continue;
        }
        match l.kind {
            // Accelerating away, a little smaller as it goes down, gone at the end.
            Exit::Fall => {
                let e = p * p;
                let d = 0.1 + 0.95 * e;
                let zt = z * (1.0 - 0.12 * e);
                let (x, y) = cam.to_screen(l.cell.x as f32 + l.way.0 * d, l.cell.y as f32 + l.way.1 * d);
                let alpha = if p < 0.8 { 1.0 } else { (1.0 - p) / 0.2 };
                let (layers, _) = wear::grown(&w.defs.thing(l.def).look_r.layers, 1.0, l.own, alpha);
                let o = (z - zt) / 2.0;
                paint(
                    s,
                    w,
                    &layers,
                    None,
                    Orient::default(),
                    l.own,
                    l.cell,
                    (x + o, y + o),
                    zt,
                    t,
                    w.defs.thing(l.def).size,
                );
            }
            // Nine pieces hop away from the worked side, shrinking.
            Exit::Crumble => {
                let e = 1.0 - (1.0 - p) * (1.0 - p);
                let hop = 0.22 * (p * std::f32::consts::PI).sin() * (1.0 - p);
                for i in 0..9 {
                    let (cx, cy) = ((i % 3) as f32 / 3.0 + 1.0 / 6.0, (i / 3) as f32 / 3.0 + 1.0 / 6.0);
                    let (dx, dy) = (cx - 0.5 + l.way.0 * 0.5, cy - 0.5 + l.way.1 * 0.5);
                    let n = (dx * dx + dy * dy).sqrt().max(0.01);
                    let k = hash2_f(l.cell.x as i64 + i, l.cell.y as i64, 81) as f32;
                    let size = z / 3.0 * (1.0 - 0.7 * p);
                    let (x, y) = cam.to_screen(
                        l.cell.x as f32 + cx + dx / n * 0.45 * e,
                        l.cell.y as f32 + cy + dy / n * 0.45 * e - hop * (0.6 + k),
                    );
                    let c = shade(l.own, 0.75 + k * 0.4);
                    s.rect(x - size / 2.0, y - size / 2.0, size, size, alpha(c, 1.0 - p * p));
                }
            }
            Exit::Pop | Exit::None => {}
        }
    }
    if z >= DETAIL_ZOOM {
        for p in &ws.parts {
            let life = p.age as f32 / p.life as f32;
            if p.kind == Kind::Dust {
                let (x, y) = cam.to_screen(p.x, p.y);
                disc(s, x, y, p.size * z * (0.6 + life * 0.8), Color::new(0.84, 0.8, 0.73, 0.32 * (1.0 - life)));
                continue;
            }
            let a = if life > 0.75 { (1.0 - life) / 0.25 } else { 1.0 };
            let size = (p.size * z).max(1.5);
            if p.h > 0.0 {
                let (x, y) = cam.to_screen(p.x + 0.03, p.y + 0.03);
                s.rect(x - size / 2.0, y - size / 2.0, size, size, Color::new(0.0, 0.0, 0.0, 0.25 * a));
            }
            let (x, y) = cam.to_screen(p.x, p.y - p.h);
            s.rect(x - size / 2.0, y - size / 2.0, size, size, alpha(p.color, p.color.a * a));
        }
    } else {
        for e in ws.sites() {
            let (Some(a), Some(&(cell, _))) = (ws.since_strike(e), w.worksites.get(&e)) else { continue };
            if a < 8 {
                let (x, y) = cam.to_screen(cell.x as f32, cell.y as f32);
                let c = alpha(site_color(w, e), 1.0 - a as f32 / 8.0);
                outline(s, x - 1.0, y - 1.0, z + 2.0, z + 2.0, 2.0, c);
            }
        }
    }
}

/// The colour a worksite is marked in: its designation's, blueprint blue
/// for a build, or the raiders' red for damage.
fn site_color(w: &World, e: Entity) -> Color {
    match w.ecs.get::<&Work>(e).ok().map(|k| k.designation) {
        Some(Some(d)) => rgb(w.defs.designations[d as usize].rgb),
        Some(None) => Color::new(0.55, 0.8, 1.0, 1.0),
        None => HOSTILE,
    }
}

/// Worksite readouts, zoomed in: a bar under each site being worked, and
/// the text under that for the UI to draw (`Chop · 62% · 3 s`, or hp).
pub fn readouts(app: &App) -> Vec<(f32, f32, String)> {
    let (w, cam, ws) = (&app.sim.world, &app.cam, &app.worksites);
    let z = cam.zoom;
    let mut out = Vec::new();
    if z < LABEL_ZOOM {
        return out;
    }
    for e in ws.sites() {
        let Some((cell, f, hurt)) = ws.readout(w, e) else { continue };
        let Some(th) = w.thing(e) else { continue };
        let (x, y) = cam.to_screen(cell.x as f32, cell.y as f32);
        let [fw, fh] = w.defs.thing(th.def).size.map(|v| v as f32);
        let (bar, y) = (z * (fw - 0.2), y + z * (fh - 1.0));
        draw_rectangle(x + z * 0.1, y + z + 3.0, bar, 3.0, Color::new(0.0, 0.0, 0.0, 0.55));
        draw_rectangle(x + z * 0.1, y + z + 3.0, bar * f, 3.0, site_color(w, e));
        let text = if hurt {
            let max = w.stat(e, "hp").unwrap_or(1.0).round() as i64;
            format!("{} · {}/{max} hp", w.defs.thing(th.def).label, th.hp.max(0))
        } else {
            let k = w.ecs.get::<&Work>(e).map(|k| *k).ok();
            let verb = match k.and_then(|k| k.designation) {
                Some(d) => w.defs.designations[d as usize].label.clone(),
                None => "Build".to_string(),
            };
            // Wall-clock time left: the worker's pace, at the game's speed.
            let left = if app.paused {
                "paused".to_string()
            } else {
                let ticks = k.map_or(0.0, |k| (k.total - k.done) as f32 / ws.pace(e).max(0.01));
                format!("{} s", (ticks / (60.0 * app.speed.max(1) as f32)).ceil())
            };
            format!("{verb} · {}% · {left}", (f * 100.0).round())
        };
        // Under the bar: above the cell is where a worker's speech goes.
        out.push((x + z * 0.1, y + z + 9.0, text));
    }
    out
}

/// Pawns, hit flashes and the field overlay.
pub fn pawns(app: &App) {
    let w = &app.sim.world;
    let defs = &w.defs;
    let cam = &app.cam;
    let z = cam.zoom;
    let (x0, y0) = cam.to_world(0.0, 0.0);
    let (x1, y1) = cam.to_world(screen_width(), screen_height());
    for &e in &w.pawns {
        let Ok(p) = w.ecs.get::<&Pawn>(e) else { continue };
        if !p.active {
            continue;
        }
        let cd = defs.creature(p.def);
        let (mut px, mut py) = pawn_pos(&p);
        if z >= DETAIL_ZOOM {
            let (lx, ly) = app.worksites.lunge(w, &p);
            (px, py) = (px + lx, py + ly);
        }
        if px < x0 - 1.0 || px > x1 + 1.0 || py < y0 - 1.0 || py > y1 + 1.0 {
            continue;
        }
        let (sx, sy) = cam.to_screen(px, py);
        let r = cd.size * z;
        disc(&mut Immediate(&app.world_atlas), sx + 1.5, sy + 2.0, r, Color::new(0.0, 0.0, 0.0, 0.3));
        disc(&mut Immediate(&app.world_atlas), sx, sy, r, rgb(cd.rgb));
        let ring = match p.faction {
            Faction::Player => Some(PLAYER),
            Faction::Hostile => Some(HOSTILE),
            Faction::Wild => None,
        };
        if let Some(rc) = ring {
            draw_circle_lines(sx, sy, r, 2.0, rc);
        }
        if app.selected == Some(e) || app.group.contains(&e) {
            draw_circle_lines(sx, sy, r + 4.0, 2.0, YELLOW);
            // Remaining path.
            let mut prev = (sx, sy);
            for step in p.path.iter().rev() {
                let s = cam.to_screen(step.x as f32 + 0.5, step.y as f32 + 0.5);
                draw_line(prev.0, prev.1, s.0, s.1, 1.5, Color::new(1.0, 1.0, 0.6, 0.35));
                prev = s;
            }
        }
        if p.drafted {
            draw_rectangle(sx - r, sy - r - 6.0, 6.0, 6.0, PLAYER);
        }
        if w.ecs.get::<&Designated>(e).is_ok() {
            draw_line(sx - r - 3.0, sy, sx + r + 3.0, sy, 1.5, HOSTILE);
            draw_line(sx, sy - r - 3.0, sx, sy + r + 3.0, 1.5, HOSTILE);
        }
        if p.hp < cd.max_hp {
            let f = (p.hp.max(0) as f32 / cd.max_hp as f32).clamp(0.0, 1.0);
            draw_rectangle(sx - r, sy + r + 3.0, r * 2.0, 3.0, Color::new(0.2, 0.0, 0.0, 0.8));
            draw_rectangle(sx - r, sy + r + 3.0, r * 2.0 * f, 3.0, Color::new(1.0 - f, f, 0.1, 1.0));
        }
        // Names, the sleep marker and bubbles are anchored UI (core:labels).
    }

    // Hit flashes.
    for (pos, tick) in &w.hits {
        let age = w.tick.saturating_sub(*tick);
        if age < 12 {
            let (sx, sy) = cam.to_screen(pos.x as f32 + 0.5, pos.y as f32 + 0.5);
            draw_circle_lines(sx, sy, z * 0.3 + age as f32, 2.0, Color::new(1.0, 1.0, 1.0, 1.0 - age as f32 / 12.0));
        }
    }

    // Field overlay, lit like the world so it reads the same way.
    if let Some(fi) = app.overlay {
        let (tx0, ty0, tx1, ty1) = visible(app);
        let fd = &defs.fields[fi];
        let (lo, hi) = (rgb(fd.rgb_low), rgb(fd.rgb_high));
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                let v = w.fields.value(defs, &w.map, fi, IVec::new(tx, ty)) as f32;
                let f = ((v - fd.range[0] as f32) / (fd.range[1] - fd.range[0]) as f32).clamp(0.0, 1.0);
                let c = Color::new(lo.r + (hi.r - lo.r) * f, lo.g + (hi.g - lo.g) * f, lo.b + (hi.b - lo.b) * f, 0.55);
                let (sx, sy) = cam.to_screen(tx as f32, ty as f32);
                draw_rectangle(sx, sy, z + 0.5, z + 0.5, c);
            }
        }
    }
}

/// Tool previews and markers, drawn after lighting so they stay readable.
pub fn world_ui(app: &App) {
    let w = &app.sim.world;
    let cam = &app.cam;
    let z = cam.zoom;

    zones(app);
    // The selected thing: pawns draw their own ring.
    if let Some(t) = app.selected.and_then(|e| w.thing(e)) {
        let (sx, sy) = cam.to_screen(t.pos.x as f32, t.pos.y as f32);
        draw_rectangle_lines(sx - 1.0, sy - 1.0, z + 2.0, z + 2.0, 2.0, YELLOW);
    }
    // Drag rectangle preview; a select drag shows once it leaves its cell.
    let (mx, my) = mouse_position();
    let dragging = app.drag_start.filter(|&a| app.tool != Tool::Select || a != cam.tile_at(mx, my));
    if let Some(a) = dragging {
        let b = cam.tile_at(mx, my);
        let (ax, ay) = (a.x.min(b.x) as f32, a.y.min(b.y) as f32);
        let (bx, by) = (a.x.max(b.x) as f32 + 1.0, a.y.max(b.y) as f32 + 1.0);
        let (s0x, s0y) = cam.to_screen(ax, ay);
        let (s1x, s1y) = cam.to_screen(bx, by);
        let c = tool_color(app);
        let outline = match app.tool {
            Tool::Build(t) => w.defs.thing(t).blocks,
            _ => false,
        };
        if outline {
            // Show exactly the cells that will get walls.
            for (ra, rb) in crate::build_rects(true, a, b) {
                let (p0x, p0y) = cam.to_screen(ra.x.min(rb.x) as f32, ra.y.min(rb.y) as f32);
                let (p1x, p1y) = cam.to_screen(ra.x.max(rb.x) as f32 + 1.0, ra.y.max(rb.y) as f32 + 1.0);
                draw_rectangle(p0x, p0y, p1x - p0x, p1y - p0y, alpha(c, 0.35));
            }
        } else {
            draw_rectangle(s0x, s0y, s1x - s0x, s1y - s0y, alpha(c, 0.18));
        }
        draw_rectangle_lines(s0x, s0y, s1x - s0x, s1y - s0y, 2.0, c);
    } else if app.tool != Tool::Select {
        let (mx, my) = mouse_position();
        let tp = cam.tile_at(mx, my);
        let (sx, sy) = cam.to_screen(tp.x as f32, tp.y as f32);
        draw_rectangle_lines(sx, sy, z, z, 2.0, tool_color(app));
    }
    order_flash(app);
}

/// Stockpiles: a light wash over each cell, and a line where a zone ends.
fn zones(app: &App) {
    let (w, cam) = (&app.sim.world, &app.cam);
    let z = cam.zoom;
    let (x0, y0, x1, y1) = visible(app);
    let zone =
        |x: i32, y: i32| w.zones.cells.get(w.map.idx(IVec::new(x, y))).copied().filter(|_| w.map.inb(IVec::new(x, y)));
    let (fill, line) = (alpha(crate::ZONE, 0.13), alpha(crate::ZONE, 0.7));
    for y in y0..=y1 {
        for x in x0..=x1 {
            let Some(id) = zone(x, y).filter(|&id| id != 0) else { continue };
            let (sx, sy) = cam.to_screen(x as f32, y as f32);
            draw_rectangle(sx, sy, z, z, fill);
            let t = (z * 0.06).clamp(1.0, 2.0);
            if zone(x, y - 1) != Some(id) {
                draw_rectangle(sx, sy, z, t, line);
            }
            if zone(x, y + 1) != Some(id) {
                draw_rectangle(sx, sy + z - t, z, t, line);
            }
            if zone(x - 1, y) != Some(id) {
                draw_rectangle(sx, sy, t, z, line);
            }
            if zone(x + 1, y) != Some(id) {
                draw_rectangle(sx + z - t, sy, t, z, line);
            }
        }
    }
}

/// Paint a look's layers over the cell whose top-left is at `at`, `z`
/// points a side. `own` is the thing's colour (or its material's).
#[allow(clippy::too_many_arguments)]
fn paint(
    s: &mut impl Sink,
    w: &World,
    layers: &[Layer],
    join: Join,
    orient: Orient,
    own: Color,
    cell: IVec,
    at: (f32, f32),
    z: f32,
    t: f32,
    span: [u32; 2],
) {
    let (sx, sy) = at;
    // A look is laid out over the thing's whole footprint (DESIGN.md §6a):
    // positions and sizes stretch on each axis, round things by the
    // shorter one so a disc stays a disc.
    let (zx, zy) = (z * span[0] as f32, z * span[1] as f32);
    let zr = zx.min(zy);
    // A fill spanning the whole cell overlaps the next one by half a point,
    // so neighbours don't show a hairline seam between them. Only a whole
    // span: a strip along the bottom edge would spill onto the cell below.
    let px = |[x, y, rw, rh]: [f32; 4]| {
        let pad = |a: f32, len: f32| if a == 0.0 && len == 1.0 { 0.5 } else { 0.0 };
        (sx + x * zx, sy + y * zy, rw * zx + pad(x, rw), rh * zy + pad(y, rh))
    };
    for l in layers {
        let base = l.color.map_or(own, |[r, g, b, a]| Color::from_rgba(r, g, b, a));
        let mut f = l.shade;
        if l.vary > 0.0 {
            f *= 1.0 - l.vary / 2.0 + hash2_f(cell.x as i64, cell.y as i64, 7) as f32 * l.vary;
        }
        let c = shade(base, f);
        let flip = orient.flip && l.into_room;
        match l.prim {
            Prim::Fill { rect, min_px } => {
                let rect = orient.rect(flip, rect);
                // Grown to `min_px` about its middle, so a seam stays centred.
                let (x, y, rw, rh) = px(rect);
                let (gw, gh) = ((min_px - rw).max(0.0), (min_px - rh).max(0.0));
                s.rect(x - gw / 2.0, y - gh / 2.0, rw + gw, rh + gh, c);
            }
            Prim::Outline { rect, width } => {
                let [x, y, rw, rh] = orient.rect(flip, rect);
                outline(s, sx + x * zx, sy + y * zy, rw * zx, rh * zy, width, c);
            }
            Prim::Disc { at, r, min_px, pulse } => {
                let [x, y] = orient.pt(flip, at);
                let f = if pulse > 0.0 { 1.0 + (t * 9.0 + cell.x as f32).sin() * pulse } else { 1.0 };
                disc(s, sx + x * zx, sy + y * zy, (r * zr).max(min_px) * f, c);
            }
            Prim::Edges { width } => edges(s, w, cell, span, join, (sx, sy), z, width, c),
            Prim::Mass => mass(s, w, cell, span, join, (sx, sy), z, c),
            Prim::Arc { at, r, from, to, width } => {
                const STEPS: usize = 8;
                let point = |k: usize| {
                    let a = from + (to - from) * k as f32 / STEPS as f32;
                    let [x, y] = orient.pt(flip, [at[0] + r * a.cos(), at[1] + r * a.sin()]);
                    (sx + x * zx, sy + y * zy)
                };
                for k in 0..STEPS {
                    let ((x0, y0), (x1, y1)) = (point(k), point(k + 1));
                    s.line(x0, y0, x1, y1, width, c);
                }
            }
            Prim::Sprite { rect, id } => {
                let (x, y, rw, rh) = px(rect);
                let slot = s.atlas().slot(id);
                s.image(x, y, rw, rh, slot, c);
            }
            Prim::Glyph { at: [x, y], size, id } => {
                let Some(g) = s.atlas().glyph(id) else { continue };
                // `size` is the em: glyphs keep their proportions and share
                // a baseline, the line's middle on the point.
                let k = size * zr / crate::atlas::GLYPH_PX;
                let (gw, gh) = (g.w * k, g.h * k);
                // A colour glyph keeps its colours; shade and vary dim it.
                let c = if g.painted { shade(Color::new(1.0, 1.0, 1.0, c.a), f) } else { c };
                s.image(sx + x * zx - gw / 2.0, sy + y * zy + g.top * k, gw, gh, g.slot, c);
            }
        }
    }
}

/// How a look sits in its cell (DESIGN.md §6c): turned across the
/// diagonal when the run it joins goes north–south (`orient = "run"`),
/// and its `into = "room"` layers mirrored when the room is to the north
/// or west. Layers are written for an east–west run with the room south.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Orient {
    pub swap: bool,
    pub flip: bool,
}

impl Orient {
    fn pt(self, flip: bool, [u, v]: [f32; 2]) -> [f32; 2] {
        let v = if flip { 1.0 - v } else { v };
        if self.swap {
            [v, u]
        } else {
            [u, v]
        }
    }
    fn rect(self, flip: bool, [x, y, w, h]: [f32; 4]) -> [f32; 4] {
        let y = if flip { 1.0 - y - h } else { y };
        if self.swap {
            [y, x, h, w]
        } else {
            [x, y, w, h]
        }
    }
}

/// Which way a one-cell look that follows its run faces: along the run its
/// joined neighbours make, and toward the enclosed side. With a room on
/// both sides, toward the smaller; with none, south or east.
fn orient_of(w: &World, cell: IVec, look: &rim_sim::look::Look, span: [u32; 2]) -> Orient {
    if !look.along_run || span != [1, 1] {
        return Orient::default();
    }
    let j = join_of(look);
    let across = joins(w, cell.offset(-1, 0), j) || joins(w, cell.offset(1, 0), j);
    let down = joins(w, cell.offset(0, -1), j) || joins(w, cell.offset(0, 1), j);
    let swap = down && !across;
    let (before, after) =
        if swap { (cell.offset(-1, 0), cell.offset(1, 0)) } else { (cell.offset(0, -1), cell.offset(0, 1)) };
    let room = |p: IVec| w.map.room_at(p).filter(|r| r.enclosed());
    let flip = match (room(before), room(after)) {
        (Some(a), Some(b)) => a.cells < b.cells,
        (Some(_), None) => true,
        _ => false,
    };
    Orient { swap, flip }
}

/// A look's join group and how round its outer corners are.
pub type Join = Option<(u16, f32)>;

pub fn join_of(look: &rim_sim::look::Look) -> Join {
    look.join.map(|g| (g, look.round))
}

/// Does the built fixture at `p` join group `join`? Plans don't: a wall
/// planned next to one doesn't open it up until it stands.
fn joins(w: &World, p: IVec, join: Join) -> bool {
    let Some((g, _)) = join else { return false };
    if w.map.fixture_at(p).is_some_and(|e| w.ecs.get::<&Blueprint>(e).is_ok()) {
        return false;
    }
    // Rock nobody has touched is terrain, and joins as its thing does.
    w.fixture_def_at(p).is_some_and(|d| w.defs.thing(d).look_r.join == Some(g))
}

/// Which sides of the footprint face a joined neighbour, and which of its
/// corners are outer corners: both sides open.
struct Sides {
    n: bool,
    e: bool,
    s: bool,
    w: bool,
}

impl Sides {
    /// A side counts as joined if its first cell faces a joined neighbour.
    fn of(w: &World, p: IVec, [sw, sh]: [u32; 2], join: Join) -> Self {
        let (sw, sh) = (sw as i32, sh as i32);
        Sides {
            n: joins(w, p.offset(0, -1), join),
            e: joins(w, p.offset(sw, 0), join),
            s: joins(w, p.offset(0, sh), join),
            w: joins(w, p.offset(-1, 0), join),
        }
    }
    /// Outer corners, NW NE SE SW: rounded when both of their sides are open.
    fn outer(&self) -> [bool; 4] {
        [!self.n && !self.w, !self.n && !self.e, !self.s && !self.e, !self.s && !self.w]
    }
}

/// A pie slice about `(cx, cy)` from angle `a0` to `a1`, in radians.
fn fan(s: &mut impl Sink, (cx, cy): (f32, f32), r: f32, a0: f32, a1: f32, c: Color) {
    let n = if r < 4.0 { 3 } else { 6 };
    let at = |k: usize| {
        let a = a0 + (a1 - a0) * k as f32 / n as f32;
        [cx + r * a.cos(), cy + r * a.sin()]
    };
    for k in 0..n {
        s.tri([[cx, cy], at(k), at(k + 1)], c);
    }
}

/// An arc about `(cx, cy)` `t` thick, as short straight lines.
fn arc(s: &mut impl Sink, (cx, cy): (f32, f32), r: f32, a0: f32, a1: f32, t: f32, c: Color) {
    let n = if r < 4.0 { 2 } else { 5 };
    let at = |k: usize| {
        let a = a0 + (a1 - a0) * k as f32 / n as f32;
        (cx + r * a.cos(), cy + r * a.sin())
    };
    for k in 0..n {
        let ((x0, y0), (x1, y1)) = (at(k), at(k + 1));
        s.line(x0, y0, x1, y1, t, c);
    }
}

/// The corner angles of a rounded corner, NW NE SE SW, as (a0, a1) about
/// its centre, and which way the centre sits from the corner.
const CORNERS: [(f32, f32, f32, f32); 4] = [
    (std::f32::consts::PI, 1.5 * std::f32::consts::PI, 1.0, 1.0),
    (1.5 * std::f32::consts::PI, std::f32::consts::TAU, -1.0, 1.0),
    (0.0, 0.5 * std::f32::consts::PI, -1.0, -1.0),
    (0.5 * std::f32::consts::PI, std::f32::consts::PI, 1.0, -1.0),
];

/// The footprint filled, a quarter at a time (DESIGN.md §6c). A quarter
/// whose two sides are both open is rounded; everything else is square, so
/// a run of joined cells reads as one body with round ends. Joined sides
/// reach half a point into the neighbour so no hairline shows between
/// them, except where the material changes, which gets a thin seam.
#[allow(clippy::too_many_arguments)]
fn mass(s: &mut impl Sink, w: &World, p: IVec, span: [u32; 2], join: Join, (sx, sy): (f32, f32), z: f32, c: Color) {
    let (zx, zy) = (z * span[0] as f32, z * span[1] as f32);
    let sides = Sides::of(w, p, span, join);
    let r = join.map_or(0.0, |(_, r)| r) * z;
    let pad = |on: bool| if on { 0.5 } else { 0.0 };
    let (x0, y0) = (sx - pad(sides.w), sy - pad(sides.n));
    let (x1, y1) = (sx + zx + pad(sides.e), sy + zy + pad(sides.s));
    let (mx, my) = (sx + zx / 2.0, sy + zy / 2.0);
    // Quarters NW NE SE SW, each from the middle out to its corner.
    let quarters = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    for (k, (round, &(ox, oy))) in sides.outer().iter().zip(&quarters).enumerate() {
        let (qx, qw) = (ox.min(mx), (ox - mx).abs());
        let (qy, qh) = (oy.min(my), (oy - my).abs());
        if !round || r <= 0.0 {
            s.rect(qx, qy, qw, qh, c);
            continue;
        }
        let r = r.min(qw).min(qh);
        let (a0, a1, dx, dy) = CORNERS[k];
        // The quarter less an r-square at its corner: a strip clear of the
        // corner's row, then the rest of that row, then the round.
        let (strip_y, strip_h) = if dy > 0.0 { (qy + r, qh - r) } else { (qy, qh - r) };
        s.rect(qx, strip_y, qw, strip_h, c);
        let row_y = if dy > 0.0 { qy } else { qy + qh - r };
        let row_x = if dx > 0.0 { qx + r } else { qx };
        s.rect(row_x, row_y, qw - r, r, c);
        fan(s, (ox + dx * r, oy + dy * r), r, a0, a1, c);
    }
    // A seam where the material changes, on the west and north sides: the
    // neighbours there are painted first, so it isn't covered.
    if span == [1, 1] {
        let made = |q: IVec| w.map.fixture_at(q).and_then(|e| w.ecs.get::<&MadeOf>(e).ok().map(|m| m.0));
        let own = made(p);
        let seam = shade(c, 0.62);
        if sides.w && made(p.offset(-1, 0)) != own {
            s.rect(sx, sy, 1.0, zy, seam);
        }
        if sides.n && made(p.offset(0, -1)) != own {
            s.rect(sx, sy, zx, 1.0, seam);
        }
    }
}

/// The border of the footprint (`span` cells from `p`, right and down),
/// a cell's length at a time, left open where it faces a joined neighbour.
/// Corners and junctions come out joined for free, and an outer corner is
/// rounded like its `mass`.
#[allow(clippy::too_many_arguments)]
fn edges(
    s: &mut impl Sink,
    w: &World,
    p: IVec,
    span: [u32; 2],
    join: Join,
    (sx, sy): (f32, f32),
    z: f32,
    t: f32,
    c: Color,
) {
    let (sw, sh) = (span[0] as i32, span[1] as i32);
    let (x0, y0, x1, y1) = (sx + 0.5, sy + 0.5, sx + z * sw as f32 - 0.5, sy + z * sh as f32 - 0.5);
    let outer = Sides::of(w, p, span, join).outer();
    let r = join.map_or(0.0, |(_, r)| r) * z;
    let r = if r > 0.0 { (r - 0.5).max(0.0) } else { 0.0 };
    // How far each corner cuts into the lines that meet at it: NW NE SE SW.
    let cut = outer.map(|o| if o { r } else { 0.0 });
    // Each step's ends, inset at the footprint's corners and cut short
    // where a corner is rounded.
    let run = |k: i32, n: i32, lo: f32, hi: f32, base: f32, cut_lo: f32, cut_hi: f32| {
        let a = (base + k as f32 * z).max(lo) + if k == 0 { cut_lo } else { 0.0 };
        let b = (base + (k + 1) as f32 * z).min(hi) - if k == n - 1 { cut_hi } else { 0.0 };
        (a, b)
    };
    for k in 0..sw {
        if !joins(w, p.offset(k, -1), join) {
            let (a, b) = run(k, sw, x0, x1, sx, cut[0], cut[1]);
            s.line(a, y0, b, y0, t, c);
        }
        if !joins(w, p.offset(k, sh), join) {
            let (a, b) = run(k, sw, x0, x1, sx, cut[3], cut[2]);
            s.line(a, y1, b, y1, t, c);
        }
    }
    for k in 0..sh {
        if !joins(w, p.offset(-1, k), join) {
            let (a, b) = run(k, sh, y0, y1, sy, cut[0], cut[3]);
            s.line(x0, a, x0, b, t, c);
        }
        if !joins(w, p.offset(sw, k), join) {
            let (a, b) = run(k, sh, y0, y1, sy, cut[1], cut[2]);
            s.line(x1, a, x1, b, t, c);
        }
    }
    if r > 0.0 {
        for (k, &(ox, oy)) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].iter().enumerate() {
            if outer[k] {
                let (a0, a1, dx, dy) = CORNERS[k];
                arc(s, (ox + dx * r, oy + dy * r), r, a0, a1, t, c);
            }
        }
    }
}

/// A ring that closes on the cell an order just landed in, so the click
/// is visibly acknowledged even when the pawn takes a moment to turn around.
const FLASH_SECS: f64 = 0.45;

fn order_flash(app: &App) {
    let Some((cell, at)) = app.order_flash else { return };
    let t = (get_time() - at) / FLASH_SECS;
    if !(0.0..1.0).contains(&t) {
        return;
    }
    let z = app.cam.zoom;
    let (cx, cy) = app.cam.to_screen(cell.x as f32 + 0.5, cell.y as f32 + 0.5);
    let r = z * (1.4 - 0.7 * t as f32);
    draw_circle_lines(cx, cy, r, 2.0, alpha(PLAYER, 1.0 - t as f32));
}

fn tool_color(app: &App) -> Color {
    app.tools.iter().find(|b| b.tool == app.tool).map_or(WHITE, |b| b.color)
}

// ================================================================== UI

/// Draw the UI's draw list. It's in physical pixels; macroquad draws in
/// logical points, so divide by the DPI factor. Glyphs are rasterised at
/// physical size, so text lands 1:1 on the screen's pixels.
/// The UI as meshes textured by the glyph atlas. Shapes sample the atlas's
/// white texel and glyphs their slots, so a whole clip region is one draw
/// call; drawing shapes untextured broke the batch at every switch between
/// a panel and its text.
struct UiBatch<'a> {
    atlas: &'a Texture2D,
    /// 1 / atlas size, to turn pixel coordinates into UVs.
    inv: f32,
    white: (f32, f32),
    verts: Vec<Vertex>,
    idx: Vec<u16>,
}

/// Flush before a mesh outgrows a draw call (see `conf()` in main.rs).
const UI_MAX_VERTS: usize = 15_000;

impl UiBatch<'_> {
    fn room(&mut self, verts: usize) {
        if self.verts.len() + verts > UI_MAX_VERTS {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.idx.is_empty() {
            return;
        }
        draw_mesh(&Mesh {
            vertices: std::mem::take(&mut self.verts),
            indices: std::mem::take(&mut self.idx),
            texture: Some(self.atlas.clone()),
        });
    }

    /// An axis-aligned quad; `uv` is a source rectangle in atlas pixels.
    fn quad(&mut self, x: f32, y: f32, w: f32, h: f32, uv: [f32; 4], c: Color) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        self.room(4);
        let n = self.verts.len() as u16;
        let (u0, v0, u1, v1) =
            (uv[0] * self.inv, uv[1] * self.inv, (uv[0] + uv[2]) * self.inv, (uv[1] + uv[3]) * self.inv);
        self.verts.extend([
            Vertex::new(x, y, 0.0, u0, v0, c),
            Vertex::new(x + w, y, 0.0, u1, v0, c),
            Vertex::new(x + w, y + h, 0.0, u1, v1, c),
            Vertex::new(x, y + h, 0.0, u0, v1, c),
        ]);
        self.idx.extend([n, n + 1, n + 2, n, n + 2, n + 3]);
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        let (u, v) = self.white;
        self.quad(x, y, w, h, [u, v, 0.0, 0.0], c);
    }

    fn tri(&mut self, a: Vec2, b: Vec2, d: Vec2, c: Color) {
        self.room(3);
        let n = self.verts.len() as u16;
        let (u, v) = (self.white.0 * self.inv, self.white.1 * self.inv);
        self.verts.extend([
            Vertex::new(a.x, a.y, 0.0, u, v, c),
            Vertex::new(b.x, b.y, 0.0, u, v, c),
            Vertex::new(d.x, d.y, 0.0, u, v, c),
        ]);
        self.idx.extend([n, n + 1, n + 2]);
    }

    /// A filled rectangle with rounded corners, from pieces that never
    /// overlap, so translucent colours stay even.
    fn rounded_rect(&mut self, [x, y, w, h]: [f32; 4], r: f32, c: Color) {
        let r = r.min(w / 2.0).min(h / 2.0);
        if r < 0.5 {
            self.rect(x, y, w, h, c);
            return;
        }
        self.rect(x + r, y, w - 2.0 * r, h, c);
        self.rect(x, y + r, r, h - 2.0 * r, c);
        self.rect(x + w - r, y + r, r, h - 2.0 * r, c);
        for (cx, cy, a0) in
            [(x + r, y + r, 180.0f32), (x + w - r, y + r, 270.0), (x + w - r, y + h - r, 0.0), (x + r, y + h - r, 90.0)]
        {
            const STEPS: usize = 4;
            let mut prev = None;
            for i in 0..=STEPS {
                let a = (a0 + 90.0 * i as f32 / STEPS as f32).to_radians();
                let p = vec2(cx + r * a.cos(), cy + r * a.sin());
                if let Some(q) = prev {
                    self.tri(vec2(cx, cy), q, p, c);
                }
                prev = Some(p);
            }
        }
    }

    /// A rounded outline `t` thick, inside the rectangle.
    fn rounded_outline(&mut self, [x, y, w, h]: [f32; 4], r: f32, t: f32, c: Color) {
        let r = r.min(w / 2.0).min(h / 2.0);
        if r < 0.5 {
            // A frame of four strips that don't overlap.
            self.rect(x, y, w, t, c);
            self.rect(x, y + h - t, w, t, c);
            self.rect(x, y + t, t, h - 2.0 * t, c);
            self.rect(x + w - t, y + t, t, h - 2.0 * t, c);
            return;
        }
        self.rect(x + r, y, w - 2.0 * r, t, c);
        self.rect(x + r, y + h - t, w - 2.0 * r, t, c);
        self.rect(x, y + r, t, h - 2.0 * r, c);
        self.rect(x + w - t, y + r, t, h - 2.0 * r, c);
        // Each corner: a quarter ring from r - t to r, in two segments.
        for (cx, cy, a0) in
            [(x + r, y + r, 180.0f32), (x + w - r, y + r, 270.0), (x + w - r, y + h - r, 0.0), (x + r, y + h - r, 90.0)]
        {
            const STEPS: usize = 2;
            let (inner, outer) = ((r - t).max(0.0), r);
            for i in 0..STEPS {
                let a = (a0 + 90.0 * i as f32 / STEPS as f32).to_radians();
                let b = (a0 + 90.0 * (i + 1) as f32 / STEPS as f32).to_radians();
                let (da, db) = (vec2(a.cos(), a.sin()), vec2(b.cos(), b.sin()));
                let o = vec2(cx, cy);
                let (p0, p1, p2, p3) = (o + da * inner, o + da * outer, o + db * inner, o + db * outer);
                self.tri(p0, p1, p2, c);
                self.tri(p2, p1, p3, c);
            }
        }
    }
}

pub fn ui(list: &[Draw], atlas: &Texture2D, white: (f32, f32), dpi: f32) {
    let s = 1.0 / dpi;
    let col = |c: [f32; 4]| Color::new(c[0], c[1], c[2], c[3]);
    let mut b = UiBatch {
        atlas,
        inv: 1.0 / atlas.width(),
        white,
        verts: Vec::with_capacity(4096),
        idx: Vec::with_capacity(6144),
    };
    for d in list {
        match d {
            Draw::Rect { rect, color, radius } => {
                b.rounded_rect(rect.map(|v| v * s), radius * s, col(*color));
            }
            Draw::Outline { rect, color, width, radius } => {
                b.rounded_outline(rect.map(|v| v * s), radius * s, width * s, col(*color));
            }
            Draw::Glyphs { quads, color } => {
                for q in quads {
                    let tint = if q.color { WHITE } else { col(*color) };
                    b.quad(q.dst[0] * s, q.dst[1] * s, q.dst[2] * s, q.dst[3] * s, q.uv, tint);
                }
            }
            Draw::Clip(r) => unsafe {
                b.flush();
                // The scissor works in framebuffer pixels: the UI's own units.
                get_internal_gl().quad_gl.scissor(Some((
                    r[0] as i32,
                    r[1] as i32,
                    r[2].ceil() as i32,
                    r[3].ceil() as i32,
                )));
            },
            Draw::Unclip => unsafe {
                b.flush();
                get_internal_gl().quad_gl.scissor(None);
            },
        }
    }
    b.flush();
    unsafe {
        get_internal_gl().quad_gl.scissor(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rim_sim::Sim;

    /// A ring of wood walls with pieces at `doors`, on cleared ground.
    fn building(s: &mut Sim, o: IVec, w: i32, h: i32, inner_x: Option<i32>, doors: &[IVec]) {
        let d = &s.world.defs;
        let (wall, door, wood) =
            (d.thing_id("wall").unwrap(), d.thing_id("door").unwrap(), d.thing_id("wood").unwrap());
        for y in -1..=h {
            for x in -1..=w {
                if let Some(e) = s.world.map.fixture_at(o.offset(x, y)) {
                    s.world.despawn_thing(e);
                }
            }
        }
        for y in 0..h {
            for x in 0..w {
                let p = o.offset(x, y);
                if x == 0 || y == 0 || x == w - 1 || y == h - 1 || Some(x) == inner_x {
                    let def = if doors.contains(&p) { door } else { wall };
                    s.world.spawn_fixture_of(def, p, false, Some(wood)).expect("a piece");
                }
            }
        }
        s.world.map.ensure_rooms();
    }

    fn orient(s: &Sim, p: IVec) -> Orient {
        let e = s.world.map.fixture_at(p).unwrap();
        let td = s.world.defs.thing(s.world.thing(e).unwrap().def);
        orient_of(&s.world, p, &td.look_r, td.size)
    }

    #[test]
    fn a_door_follows_its_wall_and_faces_the_room() {
        let mut s = Sim::new(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods")), 1).unwrap();
        let o = s.world.colony_center().unwrap().offset(12, 12);
        // Doors in the south and the west walls of a hut.
        let (south, west) = (o.offset(2, 4), o.offset(0, 2));
        building(&mut s, o, 5, 5, None, &[south, west]);
        assert_eq!(orient(&s, south), Orient { swap: false, flip: true }, "south door: room to the north");
        assert_eq!(orient(&s, west), Orient { swap: true, flip: false }, "west door: room to the east");
        // A partition door between a closet (1×3, west) and a room (2×3,
        // east) swings into the closet.
        let q = o.offset(10, 0);
        let inner = q.offset(2, 2);
        building(&mut s, q, 6, 5, Some(2), &[inner]);
        assert_eq!(orient(&s, inner), Orient { swap: true, flip: true }, "into the smaller room");
    }
}
