//! Rendering. Read-only access to the simulation.

use crate::{rgb, App, Tool};
use macroquad::prelude::*;
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
            let (x0, y0) = ((c as i32 % cx) * CHUNK, (c as i32 / cx) * CHUNK);
            let (cw, ch) = (CHUNK.min(mw - x0), CHUNK.min(mh - y0));
            tex.update_part(&Self::texels(w, x0, y0, cw, ch), x0, y0, cw, ch);
        }
    }
}

/// A filled circle, cheaper when small: under 6 px across a 20-sided circle
/// looks the same as an 8-sided one, at 24 indices instead of 60.
fn disc(x: f32, y: f32, r: f32, c: Color) {
    if r < 6.0 {
        draw_poly(x, y, 8, r, 0.0, c);
    } else {
        draw_circle(x, y, r, c);
    }
}

/// Stack counts to label, in screen points: (top-left x, y, count). The
/// client draws them with the UI's own text so they share its atlas and
/// batch (macroquad's `draw_text` broke the world's batch twice per label).
pub type Counts = Vec<(f32, f32, u32)>;

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

/// The ground, then floors, items and fixtures.
pub fn things(app: &App) -> Counts {
    let mut counts = Counts::new();
    let w = &app.sim.world;
    let defs = &w.defs;
    let cam = &app.cam;
    let z = cam.zoom;
    clear_background(Color::from_rgba(12, 14, 16, 255));

    let (tx0, ty0, tx1, ty1) = visible(app);

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
    // Floors, then items, then fixtures on top.
    for layer in 0..3 {
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                let i = (ty * w.map.w + tx) as usize;
                let Some(e) = (match layer {
                    0 => w.map.floor[i],
                    1 => w.map.item[i],
                    _ => w.map.fixture[i],
                }) else {
                    continue;
                };
                let Ok(th) = w.ecs.get::<&Thing>(e) else { continue };
                let td = defs.thing(th.def);
                // A thing built of something is drawn in that something's
                // colour, so marble arrives looking like marble with no
                // change here. Anything else keeps its def's colour.
                let c = match w.ecs.get::<&MadeOf>(e) {
                    Ok(m) => rgb(defs.thing(m.0).rgb),
                    Err(_) => rgb(td.rgb),
                };
                let (sx, sy) = cam.to_screen(tx as f32, ty as f32);
                let bp = w.ecs.get::<&Blueprint>(e).ok();
                if let Some(bp) = &bp {
                    let need: u32 = bp.cost.iter().map(|c| c.1).sum();
                    let have: u32 = bp.delivered.iter().sum();
                    let ghost = Color::new(0.45, 0.7, 1.0, 0.35);
                    draw_rectangle(sx + 1.0, sy + 1.0, z - 2.0, z - 2.0, ghost);
                    draw_rectangle_lines(sx + 1.0, sy + 1.0, z - 2.0, z - 2.0, 1.5, Color::new(0.55, 0.8, 1.0, 0.8));
                    let frac = if have < need {
                        have as f32 / need.max(1) as f32 * 0.5
                    } else {
                        0.5 + 0.5 * (1.0 - bp.work_left as f32 / bp.work.max(1) as f32)
                    };
                    draw_rectangle(sx + 2.0, sy + z - 4.0, (z - 4.0) * frac, 2.5, Color::new(0.6, 0.9, 1.0, 0.9));
                    continue;
                }
                let look = &td.look_r;
                let layers = match w.ecs.get::<&Regrow>(e) {
                    Ok(_) if !look.regrowing.is_empty() => &look.regrowing,
                    _ => &look.layers,
                };
                paint(w, layers, look.join, c, IVec::new(tx, ty), (sx, sy), z, t);
                if z >= 22.0 && th.count > 1 {
                    counts.push((sx + z * 0.22, sy + z * 0.95 - 12.0, th.count));
                }
                if let Ok(d) = w.ecs.get::<&Designated>(e) {
                    let dc = rgb(defs.designations[d.0 as usize].rgb);
                    disc(sx + z * 0.82, sy + z * 0.18, z * 0.13 + 1.0, BLACK);
                    disc(sx + z * 0.82, sy + z * 0.18, z * 0.13, dc);
                }
            }
        }
    }
    counts
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
        let (px, py) = pawn_pos(&p);
        if px < x0 - 1.0 || px > x1 + 1.0 || py < y0 - 1.0 || py > y1 + 1.0 {
            continue;
        }
        let (sx, sy) = cam.to_screen(px, py);
        let r = cd.size * z;
        disc(sx + 1.5, sy + 2.0, r, Color::new(0.0, 0.0, 0.0, 0.3));
        disc(sx, sy, r, rgb(cd.rgb));
        let ring = match p.faction {
            Faction::Player => Some(PLAYER),
            Faction::Hostile => Some(HOSTILE),
            Faction::Wild => None,
        };
        if let Some(rc) = ring {
            draw_circle_lines(sx, sy, r, 2.0, rc);
        }
        if app.selected == Some(e) {
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

    // Drag rectangle preview.
    if let Some(a) = app.drag_start {
        let (mx, my) = mouse_position();
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

/// Paint a look's layers over the cell whose top-left is at `at`, `z`
/// points a side. `own` is the thing's colour (or its material's).
#[allow(clippy::too_many_arguments)]
fn paint(w: &World, layers: &[Layer], join: Option<u16>, own: Color, cell: IVec, at: (f32, f32), z: f32, t: f32) {
    let (sx, sy) = at;
    // A fill spanning the whole cell overlaps the next one by half a point,
    // so neighbours don't show a hairline seam between them. Only a whole
    // span: a strip along the bottom edge would spill onto the cell below.
    let px = |[x, y, rw, rh]: [f32; 4]| {
        let pad = |a: f32, len: f32| if a == 0.0 && len == 1.0 { 0.5 } else { 0.0 };
        (sx + x * z, sy + y * z, rw * z + pad(x, rw), rh * z + pad(y, rh))
    };
    for l in layers {
        let base = l.color.map_or(own, |[r, g, b, a]| Color::from_rgba(r, g, b, a));
        let mut f = l.shade;
        if l.vary > 0.0 {
            f *= 1.0 - l.vary / 2.0 + hash2_f(cell.x as i64, cell.y as i64, 7) as f32 * l.vary;
        }
        let c = shade(base, f);
        match l.prim {
            Prim::Fill { rect, min_px } => {
                // Grown to `min_px` about its middle, so a seam stays centred.
                let (x, y, rw, rh) = px(rect);
                let (gw, gh) = ((min_px - rw).max(0.0), (min_px - rh).max(0.0));
                draw_rectangle(x - gw / 2.0, y - gh / 2.0, rw + gw, rh + gh, c);
            }
            Prim::Outline { rect, width } => {
                let [x, y, rw, rh] = rect;
                draw_rectangle_lines(sx + x * z, sy + y * z, rw * z, rh * z, width, c);
            }
            Prim::Disc { at: [x, y], r, min_px, pulse } => {
                let f = if pulse > 0.0 { 1.0 + (t * 9.0 + cell.x as f32).sin() * pulse } else { 1.0 };
                disc(sx + x * z, sy + y * z, (r * z).max(min_px) * f, c);
            }
            Prim::Edges { width } => edges(w, cell, join, (sx, sy), z, width, c),
        }
    }
}

/// Does the built fixture at `p` join group `join`? Plans don't: a wall
/// planned next to one doesn't open it up until it stands.
fn joins(w: &World, p: IVec, join: Option<u16>) -> bool {
    let Some(g) = join else { return false };
    let Some(e) = w.map.fixture_at(p) else { return false };
    if w.ecs.get::<&Blueprint>(e).is_ok() {
        return false;
    }
    w.ecs.get::<&Thing>(e).is_ok_and(|t| w.defs.thing(t.def).look_r.join == Some(g))
}

/// The cell's border on the sides that don't face a joined neighbour.
/// Corners and junctions come out joined for free.
fn edges(w: &World, p: IVec, join: Option<u16>, (sx, sy): (f32, f32), z: f32, t: f32, c: Color) {
    let (x0, y0, x1, y1) = (sx + 0.5, sy + 0.5, sx + z - 0.5, sy + z - 0.5);
    if !joins(w, p.offset(0, -1), join) {
        draw_line(x0, y0, x1, y0, t, c);
    }
    if !joins(w, p.offset(0, 1), join) {
        draw_line(x0, y1, x1, y1, t, c);
    }
    if !joins(w, p.offset(-1, 0), join) {
        draw_line(x0, y0, x0, y1, t, c);
    }
    if !joins(w, p.offset(1, 0), join) {
        draw_line(x1, y0, x1, y1, t, c);
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
