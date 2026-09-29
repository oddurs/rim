//! What doesn't move, drawn from the GPU (DESIGN.md §8).
//!
//! Floors, items and fixtures are painted once per chunk, in map cells, and
//! a region of chunks' paint is joined into vertex buffers that live on the
//! GPU and are redrawn from there every frame: a draw call per region and
//! layer, and no vertices built or uploaded. A chunk is painted again when
//! its things revision moves (the map bumps it for anything drawn there),
//! and its region joined again. Line widths and minimum sizes are in screen
//! points, so the zoom matters too: while it moves, chunks are drawn scaled
//! from the zoom they were painted at, and painted again, a few a frame,
//! once it settles or has drifted too far to pass for the same picture.
//!
//! Worksites (things being worked right now, DESIGN.md §6b) and animated
//! looks change every frame, so they stay out of the buffers and are drawn
//! by `draw::things` each frame. A plan nobody is building is as still as a
//! rock and is cached with it; the sim touches the map when work on a site
//! starts or stops, which moves it between the two.

use crate::atlas::{Slot, WorldAtlas};
use crate::draw::{self, Sink};
use crate::Cam;
use macroquad::miniquad::*;
use macroquad::prelude::{get_internal_gl, screen_height, screen_width, Color};
use rim_sim::map::CHUNK;
use rim_sim::world::{Thing, World};
use rim_sim::IVec;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vert {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [u8; 4],
}

/// u16 indices: a buffer holds at most this many vertices.
const MAX_VERTS: usize = u16::MAX as usize;

/// Geometry painted in screen points relative to a chunk's top-left
/// corner at zoom `z`, kept in map cells, by atlas page. Primitives sample
/// page 0's white block.
pub struct Builder<'a> {
    atlas: &'a WorldAtlas,
    parts: Vec<(usize, Vec<Vert>, Vec<u16>)>,
    /// The chunk's corner, in cells, and the points a cell it is painted at.
    corner: (f32, f32),
    z: f32,
}

impl<'a> Builder<'a> {
    fn new(atlas: &'a WorldAtlas, corner: (f32, f32), z: f32) -> Self {
        Builder { atlas, parts: Vec::new(), corner, z }
    }

    /// A painted point, in map cells.
    fn cell(&self, [x, y]: [f32; 2]) -> [f32; 2] {
        [self.corner.0 + x / self.z, self.corner.1 + y / self.z]
    }

    /// The part to append to: the last one, if it is on `page` and has
    /// room. Only ever the last, so parts draw in the order painted.
    fn room(&mut self, page: usize, verts: usize) -> (&mut Vec<Vert>, &mut Vec<u16>) {
        if self.parts.last().is_none_or(|p| p.0 != page || p.1.len() + verts > MAX_VERTS) {
            self.parts.push((page, Vec::new(), Vec::new()));
        }
        let (_, v, i) = self.parts.last_mut().expect("just pushed");
        (v, i)
    }

    fn quad(&mut self, page: usize, p: [[f32; 2]; 4], uv: [[f32; 2]; 4], c: Color) {
        let color: [u8; 4] = c.into();
        let p = p.map(|q| self.cell(q));
        let (v, i) = self.room(page, 4);
        let n = v.len() as u16;
        v.extend((0..4).map(|k| Vert { pos: p[k], uv: uv[k], color }));
        i.extend([n, n + 1, n + 2, n, n + 2, n + 3]);
    }

    fn solid(&mut self, p: [[f32; 2]; 4], c: Color) {
        let w = self.atlas.white(0);
        self.quad(0, p, [w; 4], c);
    }
}

impl Sink for Builder<'_> {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        self.solid([[x, y], [x + w, y], [x + w, y + h], [x, y + h]], c);
    }

    /// Like macroquad's `draw_poly`: a fan of `sides` triangles.
    fn poly(&mut self, x: f32, y: f32, sides: u8, r: f32, c: Color) {
        let color: [u8; 4] = c.into();
        let uv = self.atlas.white(0);
        let ((cx, cy), z) = (self.corner, self.z);
        let cell = |px: f32, py: f32| [cx + px / z, cy + py / z];
        let (v, i) = self.room(0, sides as usize + 2);
        let n = v.len() as u16;
        v.push(Vert { pos: cell(x, y), uv, color });
        for k in 0..=sides {
            let a = k as f32 / sides as f32 * std::f32::consts::TAU;
            v.push(Vert { pos: cell(x + r * a.cos(), y + r * a.sin()), uv, color });
            if k != sides {
                i.extend([n, n + k as u16 + 1, n + k as u16 + 2]);
            }
        }
    }

    /// Like macroquad's `draw_line`: a quad `t` wide about the segment.
    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, t: f32, c: Color) {
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len = (dx * dx + dy * dy).sqrt() / (t * 0.5);
        if len < f32::EPSILON {
            return;
        }
        let (tx, ty) = (-dy / len, dx / len);
        self.solid([[x0 + tx, y0 + ty], [x0 - tx, y0 - ty], [x1 - tx, y1 - ty], [x1 + tx, y1 + ty]], c);
    }

    fn tri(&mut self, p: [[f32; 2]; 3], c: Color) {
        let color: [u8; 4] = c.into();
        let uv = self.atlas.white(0);
        let p = p.map(|q| self.cell(q));
        let (v, i) = self.room(0, 3);
        let n = v.len() as u16;
        v.extend(p.map(|pos| Vert { pos, uv, color }));
        i.extend([n, n + 1, n + 2]);
    }

    fn atlas(&self) -> &WorldAtlas {
        self.atlas
    }

    fn image(&mut self, x: f32, y: f32, w: f32, h: f32, s: Slot, c: Color) {
        let [u0, v0, u1, v1] = s.uv;
        self.quad(
            s.page,
            [[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
            [[u0, v0], [u1, v0], [u1, v1], [u0, v1]],
            c,
        );
    }
}

/// One atlas page's run of a chunk's paint, in map cells.
struct Paint {
    page: usize,
    verts: Vec<Vert>,
    indices: Vec<u16>,
}

/// Chunks joined into one buffer per layer and page run: `REGION` by
/// `REGION` of them.
const REGION: i32 = 2;

/// A region's buffers on the GPU, by layer.
#[derive(Default)]
struct Region {
    parts: [Vec<Part>; 3],
    /// A chunk in it was painted since it was last joined.
    stale: bool,
}

/// A chunk's indices in a joined buffer: chunk, first index, count.
type Run = (usize, i32, i32);

/// One buffer: a run of paint on one page, from one or more chunks in turn.
struct Part {
    bindings: Bindings,
    /// Each chunk's indices in it. A region partly on screen draws only its
    /// chunks that are.
    runs: Vec<Run>,
}

impl Region {
    fn free(&mut self, ctx: &mut dyn RenderingBackend) {
        for p in self.parts.iter_mut().flat_map(std::mem::take) {
            ctx.delete_buffer(p.bindings.vertex_buffers[0]);
            ctx.delete_buffer(p.bindings.index_buffer);
        }
    }
}

#[derive(Default)]
struct Chunk {
    /// The things revision and zoom it was painted at.
    built: Option<(u64, f32)>,
    /// Per layer: floors, items, fixtures.
    paint: [Vec<Paint>; 3],
    /// Cells drawn each frame instead (worksites, animated looks), by layer.
    live: [Vec<IVec>; 3],
    /// Stack counts to label: cell and count.
    counts: Vec<(IVec, u32)>,
    /// Something anchored here reaches past its right or bottom edge, so it
    /// is drawn when the chunk beside or below it is on screen.
    spills: bool,
    /// Something here faces the room beside it (a door's swing): the room
    /// rebuild it was drawn for, since rooms can change with no wall here.
    rooms_seen: Option<u64>,
}

pub struct Meshes {
    pipeline: Option<Pipeline>,
    /// Every level's chunks, as `Map::level_chunks` numbers them. Only the
    /// viewed level and the ones beside it hold paint.
    chunks: Vec<Chunk>,
    /// Every level's regions, level by level, row by row.
    regions: Vec<Region>,
    /// Which chunks the layer being drawn shows, by chunk.
    shown: Vec<bool>,
    /// The level the buffers were last prepared for.
    level: Option<i32>,
    /// Chunks on screen this frame (from `prepare`).
    visible: Vec<usize>,
    /// Chunks of the level below that show through air on screen.
    below: Vec<usize>,
    /// The zoom last frame, and how many frames it has held.
    zoom: (f32, u32),
    /// Last frame, for the render bench and the profiler.
    pub calls: usize,
    pub indices: usize,
    pub rebuilt: usize,
    /// Regions joined again last frame, and the time it took (µs): what a
    /// change costs on top of painting its chunk.
    pub joined: usize,
    pub join_us: f64,
    /// Time painting chunks last frame (µs).
    pub paint_us: f64,
    /// Time in `draw_layer` handing buffers to GL (µs).
    pub submit_us: f64,
}

impl Default for Meshes {
    fn default() -> Self {
        Meshes {
            pipeline: None,
            chunks: Vec::new(),
            regions: Vec::new(),
            shown: Vec::new(),
            level: None,
            visible: Vec::new(),
            below: Vec::new(),
            zoom: (0.0, 0),
            calls: 0,
            indices: 0,
            rebuilt: 0,
            joined: 0,
            join_us: 0.0,
            paint_us: 0.0,
            submit_us: 0.0,
        }
    }
}

const VERTEX: &str = "#version 100
attribute vec2 pos;
attribute vec2 uv0;
attribute vec4 color0;
varying lowp vec4 color;
// A texel on a 4096 page needs more than mediump's 10 bits.
#ifdef GL_FRAGMENT_PRECISION_HIGH
varying highp vec2 uv;
#else
varying mediump vec2 uv;
#endif
uniform vec2 origin;
uniform vec2 screen;
uniform float zoom;
uniform float flip;
void main() {
    vec2 p = (pos * zoom + origin) / screen * 2.0 - 1.0;
    gl_Position = vec4(p.x, p.y * flip, 0.0, 1.0);
    color = color0 / 255.0;
    uv = uv0;
}";

const FRAGMENT: &str = "#version 100
varying lowp vec4 color;
#ifdef GL_FRAGMENT_PRECISION_HIGH
varying highp vec2 uv;
#else
varying mediump vec2 uv;
#endif
uniform sampler2D tex;
void main() {
    gl_FragColor = texture2D(tex, uv) * color;
}";

#[repr(C)]
struct Uniforms {
    /// Where the map's corner is on screen.
    origin: [f32; 2],
    screen: [f32; 2],
    /// Points a cell.
    zoom: f32,
    /// -1 to the screen, 1 into a render target (GL's rows run upward).
    flip: f32,
}

/// Frames the zoom must hold before stale chunks are rebuilt at it.
const SETTLE_FRAMES: u32 = 8;
/// Past this ratio of zooms a scaled chunk no longer passes (lines twice
/// as thick), so it is rebuilt even mid-gesture.
const MAX_SCALE: f32 = 2.0;
/// Rebuilding for the zoom stops for the frame after this long; the rest
/// wait, drawn scaled. Content changes always rebuild, and so does a chunk
/// scaled past `MAX_SCALE` once the budget allows.
const ZOOM_BUDGET_US: f64 = 1500.0;

/// The index spans to draw of a joined buffer: its runs of chunks on
/// screen, runs that follow on in the buffer drawn as one.
fn spans(runs: &[Run], shown: &[bool], mut draw: impl FnMut(i32, i32)) {
    let mut span: Option<(i32, i32)> = None;
    for &(_, first, n) in runs.iter().filter(|r| shown[r.0]) {
        span = match span {
            Some((s, k)) if s + k == first => Some((s, k + n)),
            Some((s, k)) => {
                draw(s, k);
                Some((first, n))
            }
            None => Some((first, n)),
        };
    }
    if let Some((s, k)) = span {
        draw(s, k);
    }
}

/// Should this thing be drawn each frame rather than cached?
fn live(w: &World, e: rim_sim::hecs::Entity) -> bool {
    w.is_worksite(e) || w.ecs.get::<&Thing>(e).is_ok_and(|t| w.defs.thing(t.def).look_r.animated())
}

impl Meshes {
    fn pipeline(ctx: &mut dyn RenderingBackend) -> Pipeline {
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
                ShaderMeta {
                    images: vec!["tex".to_string()],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("origin", UniformType::Float2),
                            UniformDesc::new("screen", UniformType::Float2),
                            UniformDesc::new("zoom", UniformType::Float1),
                            UniformDesc::new("flip", UniformType::Float1),
                        ],
                    },
                },
            )
            // The shader is GLSL 100, the lowest every backend compiles.
            .expect("the chunk shader compiles");
        ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("pos", VertexFormat::Float2),
                VertexAttribute::new("uv0", VertexFormat::Float2),
                VertexAttribute::new("color0", VertexFormat::Byte4),
            ],
            shader,
            PipelineParams {
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
        )
    }

    /// The region chunk `c` is joined into, on a map `chunks` wide and high.
    fn region_of((cx, cy): (i32, i32), c: usize) -> usize {
        let per = (cx * cy) as usize;
        let (rx, ry) = ((cx + REGION - 1) / REGION, (cy + REGION - 1) / REGION);
        let (slot, i) = (c / per, (c % per) as i32);
        slot * (rx * ry) as usize + ((i / cx / REGION) * rx + i % cx / REGION) as usize
    }

    /// The chunks of region `r`, in the order they are joined.
    fn chunks_of((cx, cy): (i32, i32), r: usize) -> impl Iterator<Item = usize> {
        let (rx, ry) = ((cx + REGION - 1) / REGION, (cy + REGION - 1) / REGION);
        let (slot, i) = (r / (rx * ry) as usize, r as i32 % (rx * ry));
        let base = slot * (cx * cy) as usize;
        let (x0, y0) = (i % rx * REGION, i / rx * REGION);
        (y0..(y0 + REGION).min(cy))
            .flat_map(move |y| (x0..(x0 + REGION).min(cx)).map(move |x| base + (y * cx + x) as usize))
    }

    /// Paint chunk `c` again, and mark its region to be joined.
    fn build(&mut self, w: &World, atlas: &WorldAtlas, c: usize, z: f32, t: f32) {
        let start = std::time::Instant::now();
        let IVec { x: x0, y: y0, z: z0 } = w.map.chunk_origin(c);
        self.regions[Self::region_of(w.map.chunks(), c)].stale = true;
        let chunk = &mut self.chunks[c];
        chunk.live = Default::default();
        chunk.counts.clear();
        chunk.spills = false;
        chunk.rooms_seen = None;
        for layer in 0..3 {
            let mut b = Builder::new(atlas, (x0 as f32, y0 as f32), z);
            for y in y0..(y0 + CHUNK).min(w.map.h) {
                for x in x0..(x0 + CHUNK).min(w.map.w) {
                    let cell = IVec::at(x, y, z0);
                    let i = w.map.idx(cell);
                    let Some(e) = w.map.layers_at(i)[layer] else {
                        // Rock with nobody on it is terrain, drawn as its thing.
                        if let Some(thing) = (layer == 2).then(|| w.solid_at(cell).and_then(|s| s.thing_r)).flatten() {
                            let at = ((x - x0) as f32 * z, (y - y0) as f32 * z);
                            draw::rock(&mut b, w, thing, cell, at, z, t);
                        }
                        continue;
                    };
                    if let Some(t) = w.thing(e).filter(|t| t.pos == cell) {
                        let td = w.defs.thing(t.def);
                        let [sw, sh] = td.size;
                        chunk.spills |= x + sw as i32 > x0 + CHUNK || y + sh as i32 > y0 + CHUNK;
                        if td.look_r.reads_rooms() {
                            chunk.rooms_seen = Some(w.map.room_rebuilds);
                        }
                    }
                    if live(w, e) {
                        chunk.live[layer].push(cell);
                        continue;
                    }
                    let at = ((x - x0) as f32 * z, (y - y0) as f32 * z);
                    if let Some(n) = draw::thing(&mut b, w, e, cell, at, z, t, Default::default()) {
                        chunk.counts.push((cell, n));
                    }
                }
            }
            chunk.paint[layer] = b
                .parts
                .into_iter()
                .filter(|(_, _, i)| !i.is_empty())
                .map(|(page, verts, indices)| Paint { page, verts, indices })
                .collect();
        }
        chunk.built = Some((w.map.things_rev(c), z));
        self.rebuilt += 1;
        self.paint_us += start.elapsed().as_secs_f64() * 1e6;
    }

    /// Paint chunk `c` again next frame, as a change there would: for the
    /// render bench.
    pub fn repaint(&mut self, c: usize) {
        if let Some(ch) = self.chunks.get_mut(c) {
            ch.built = None;
        }
    }

    /// Join the paint of region `r`'s chunks into fresh buffers: per layer,
    /// each chunk's runs in turn, a run on the page the last one ended on
    /// carrying on in the same buffer while it has room.
    fn join(&mut self, ctx: &mut dyn RenderingBackend, w: &World, atlas: &WorldAtlas, r: usize) {
        let start = std::time::Instant::now();
        self.regions[r].free(ctx);
        for layer in 0..3 {
            let mut joined: Vec<(Paint, Vec<Run>)> = Vec::new();
            for c in Self::chunks_of(w.map.chunks(), r) {
                for p in &self.chunks[c].paint[layer] {
                    let fits = joined
                        .last()
                        .is_some_and(|j| j.0.page == p.page && j.0.verts.len() + p.verts.len() <= MAX_VERTS);
                    if !fits {
                        joined.push((Paint { page: p.page, verts: Vec::new(), indices: Vec::new() }, Vec::new()));
                    }
                    let (Paint { verts, indices, .. }, runs) = joined.last_mut().expect("just pushed");
                    let (base, first) = (verts.len() as u16, indices.len() as i32);
                    verts.extend_from_slice(&p.verts);
                    indices.extend(p.indices.iter().map(|i| i + base));
                    match runs.last_mut() {
                        Some(run) if run.0 == c => run.2 += p.indices.len() as i32,
                        _ => runs.push((c, first, p.indices.len() as i32)),
                    }
                }
            }
            self.regions[r].parts[layer] = joined
                .into_iter()
                .map(|(Paint { page, verts, indices }, runs)| Part {
                    bindings: Bindings {
                        vertex_buffers: vec![ctx.new_buffer(
                            BufferType::VertexBuffer,
                            BufferUsage::Immutable,
                            BufferSource::slice(&verts),
                        )],
                        index_buffer: ctx.new_buffer(
                            BufferType::IndexBuffer,
                            BufferUsage::Immutable,
                            BufferSource::slice(&indices),
                        ),
                        images: vec![atlas.pages[page].raw_miniquad_id()],
                    },
                    runs,
                })
                .collect();
        }
        self.regions[r].stale = false;
        self.joined += 1;
        self.join_us += start.elapsed().as_secs_f64() * 1e6;
    }

    /// Find this frame's visible chunks on `level`, and the level below's
    /// under its air, and rebuild the stale ones.
    pub fn prepare(&mut self, w: &World, atlas: &WorldAtlas, cam: &Cam, level: i32, t: f32) {
        // SAFETY: macroquad's context outlives the frame, and nothing else
        // holds it while the world draws.
        let gl = unsafe { get_internal_gl() };
        let ctx = gl.quad_context;
        let (cx, cy) = w.map.chunks();
        let total = w.map.levels().map(|z| w.map.level_chunks(z).end).max().unwrap_or(0);
        if self.chunks.len() != total {
            for r in &mut self.regions {
                r.free(ctx);
            }
            self.chunks = (0..total).map(|_| Chunk::default()).collect();
            let regions = if total == 0 { 0 } else { Self::region_of(w.map.chunks(), total - 1) + 1 };
            self.regions = (0..regions).map(|_| Region::default()).collect();
            self.shown = vec![false; total];
            self.level = None;
        }
        if self.level != Some(level) {
            // Lighting crossfades the level above as well as below
            // (DESIGN.md §6e), so both neighbours keep their buffers.
            for z in w.map.levels().filter(|z| (z - level).abs() > 1) {
                for c in w.map.level_chunks(z) {
                    self.chunks[c].paint = Default::default();
                    self.chunks[c].built = None;
                    self.regions[Self::region_of(w.map.chunks(), c)].free(ctx);
                }
            }
            self.level = Some(level);
        }
        let base = w.map.level_chunks(level).start;
        let (wx0, wy0) = cam.to_world(0.0, 0.0);
        let (wx1, wy1) = cam.to_world(screen_width(), screen_height());
        let span = |a: f32, b: f32, n: i32| {
            ((a / CHUNK as f32).floor().max(0.0) as i32, ((b / CHUNK as f32).floor() as i32).min(n - 1))
        };
        let ((c0x, c1x), (c0y, c1y)) = (span(wx0, wx1, cx), span(wy0, wy1, cy));
        self.visible = (c0y..=c1y).flat_map(|y| (c0x..=c1x).map(move |x| base + (y * cx + x) as usize)).collect();
        // Below, only chunks under an air cell on screen: a level with no
        // pits costs nothing.
        self.below.clear();
        if w.map.levels().contains(&(level - 1)) {
            let under = w.map.level_chunks(level - 1).start;
            for &i in w.map.air_cells(level) {
                let p = w.map.pos(i as usize);
                let (x, y) = (p.x / CHUNK, p.y / CHUNK);
                let c = under + (y * cx + x) as usize;
                if (c0x..=c1x).contains(&x) && (c0y..=c1y).contains(&y) {
                    self.below.push(c);
                }
            }
            self.below.sort_unstable();
            self.below.dedup();
        }

        if self.pipeline.is_none() {
            self.pipeline = Some(Self::pipeline(ctx));
        }
        self.rebuilt = 0;
        (self.calls, self.indices, self.submit_us) = (0, 0, 0.0);
        (self.joined, self.join_us, self.paint_us) = (0, 0.0, 0.0);
        self.zoom = if self.zoom.0 == cam.zoom { (cam.zoom, self.zoom.1 + 1) } else { (cam.zoom, 0) };
        let settled = self.zoom.1 >= SETTLE_FRAMES;
        let start = std::time::Instant::now();
        for k in 0..self.visible.len() + self.below.len() {
            let c = self.visible.get(k).copied().unwrap_or_else(|| self.below[k - self.visible.len()]);
            let changed = match self.chunks[c].built {
                None => true,
                Some((r, _)) if r != w.map.things_rev(c) => true,
                _ => self.chunks[c].rooms_seen.is_some_and(|r| r != w.map.room_rebuilds),
            };
            let rezoom = self.chunks[c].built.is_some_and(|(_, z)| {
                let far = (cam.zoom / z).max(z / cam.zoom) > MAX_SCALE;
                z != cam.zoom && (settled || far) && start.elapsed().as_secs_f64() * 1e6 < ZOOM_BUDGET_US
            });
            if changed {
                self.build(w, atlas, c, cam.zoom, t);
            } else if rezoom {
                // A region's chunks on screen at once, joined once and inside
                // the budget: a chunk at a time would join it up to four
                // times. Its chunks off screen keep their zoom until seen.
                let r = Self::region_of(w.map.chunks(), c);
                for cc in Self::chunks_of(w.map.chunks(), r) {
                    let seen = self.visible.contains(&cc) || self.below.contains(&cc);
                    if seen && self.chunks[cc].built.is_some_and(|(_, z)| z != cam.zoom) {
                        self.build(w, atlas, cc, cam.zoom, t);
                    }
                }
                self.join(ctx, w, atlas, r);
            }
        }
        // A thing is drawn from its anchor, and its footprint reaches right
        // and down (at most `MAX_SIZE` cells, less than a chunk): a chunk
        // just above or left of the view may hold one reaching into it. It
        // is drawn only if it does.
        if w.defs.things.iter().any(|d| d.size != [1, 1]) {
            let left = (c0x > 0).then(|| (c0y.max(1) - 1..=c1y).map(|y| base + (y * cx + c0x - 1) as usize));
            let above = (c0y > 0).then(|| (c0x..=c1x).map(|x| base + ((c0y - 1) * cx + x) as usize));
            for c in left.into_iter().flatten().chain(above.into_iter().flatten()) {
                // Only when what's in it changed: at another zoom it is drawn
                // scaled, like any chunk, and it's off screen anyway.
                if self.chunks[c].built.is_none_or(|(r, _)| r != w.map.things_rev(c)) {
                    self.build(w, atlas, c, cam.zoom, t);
                }
                if self.chunks[c].spills {
                    self.visible.push(c);
                }
            }
        }
        for r in 0..self.regions.len() {
            if self.regions[r].stale {
                self.join(ctx, w, atlas, r);
            }
        }
    }

    /// Draw one layer (floors, items, fixtures) of every visible chunk from
    /// its region's buffers, or with `below` of the level below's chunks
    /// under air: a call per region and page run, or per run of chunks on
    /// screen where a region is only partly. Whatever macroquad has batched
    /// so far goes first, so the layers below stay below.
    pub fn draw_layer(&mut self, w: &World, cam: &Cam, layer: usize, target: Option<RenderPass>, below: bool) {
        let start = std::time::Instant::now();
        // SAFETY: as in `prepare`.
        let mut gl = unsafe { get_internal_gl() };
        gl.flush();
        let ctx = gl.quad_context;
        let Some(pipeline) = &self.pipeline else { return };
        let screen = [screen_width(), screen_height()];
        match target {
            Some(pass) => ctx.begin_pass(Some(pass), PassAction::Nothing),
            None => ctx.begin_default_pass(PassAction::Nothing),
        }
        let flip = if target.is_some() { 1.0 } else { -1.0 };
        ctx.apply_pipeline(pipeline);
        let origin = cam.to_screen(0.0, 0.0);
        ctx.apply_uniforms(UniformsSource::table(&Uniforms {
            origin: [origin.0, origin.1],
            screen,
            zoom: cam.zoom,
            flip,
        }));
        let list = if below { &self.below } else { &self.visible };
        let mut regions: Vec<usize> = list.iter().map(|&c| Self::region_of(w.map.chunks(), c)).collect();
        regions.sort_unstable();
        regions.dedup();
        for &c in list {
            self.shown[c] = true;
        }
        for r in regions {
            for p in &self.regions[r].parts[layer] {
                ctx.apply_bindings(&p.bindings);
                // A span may start mid-buffer: fine on GL, which rim forces
                // on macOS; miniquad's Metal backend asserts it starts at 0.
                spans(&p.runs, &self.shown, |first, n| {
                    ctx.draw(first, n, 1);
                    self.calls += 1;
                    self.indices += n as usize;
                });
            }
        }
        for &c in list {
            self.shown[c] = false;
        }
        ctx.end_render_pass();
        self.submit_us += start.elapsed().as_secs_f64() * 1e6;
    }

    /// Cells of `layer` in the visible chunks to draw live this frame.
    pub fn live(&self, layer: usize) -> impl Iterator<Item = IVec> + '_ {
        self.visible.iter().flat_map(move |&c| self.chunks[c].live[layer].iter().copied())
    }

    /// Is chunk `c` drawn this frame?
    pub fn drawn(&self, c: usize) -> bool {
        self.visible.contains(&c) || self.below.contains(&c)
    }

    /// How many things the visible chunks leave to be drawn live.
    pub fn live_count(&self) -> usize {
        self.visible.iter().map(|&c| self.chunks[c].live.iter().map(Vec::len).sum::<usize>()).sum()
    }

    /// Stacks to label in the visible chunks: cell and count.
    pub fn counts(&self) -> impl Iterator<Item = (IVec, u32)> + '_ {
        self.visible.iter().flat_map(|&c| self.chunks[c].counts.iter().copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_share_out_every_chunk_once_on_every_level() {
        // Odd sizes leave regions part empty at the right and bottom.
        for chunks @ (cx, cy) in [(8, 8), (3, 5), (1, 1)] {
            let total = (cx * cy) as usize * 3;
            let regions = Meshes::region_of(chunks, total - 1) + 1;
            let mut seen = vec![0; total];
            for r in 0..regions {
                for c in Meshes::chunks_of(chunks, r) {
                    assert_eq!(Meshes::region_of(chunks, c), r, "{chunks:?}: chunk {c} is in region {r}");
                    seen[c] += 1;
                }
            }
            assert!(seen.iter().all(|&n| n == 1), "{chunks:?}: {seen:?}");
        }
        // A region is 2 by 2 chunks: the whole 8 by 8 map is 16 of them.
        assert_eq!(Meshes::region_of((8, 8), 63), 15);
        assert_eq!(Meshes::chunks_of((8, 8), 0).collect::<Vec<_>>(), [0, 1, 8, 9]);
    }

    #[test]
    fn chunks_on_screen_that_follow_on_are_one_draw() {
        let runs = [(0, 0, 6), (1, 6, 12), (8, 18, 3), (9, 21, 9)];
        let draws = |shown: &[usize]| {
            let mut on = vec![false; 10];
            for &c in shown {
                on[c] = true;
            }
            let mut out = Vec::new();
            spans(&runs, &on, |first, n| out.push((first, n)));
            out
        };
        assert_eq!(draws(&[0, 1, 8, 9]), [(0, 30)], "a region wholly on screen is one call");
        assert_eq!(draws(&[1, 9]), [(6, 12), (21, 9)], "a gap splits it");
        assert_eq!(draws(&[8, 9]), [(18, 12)]);
        assert_eq!(draws(&[]), []);
    }
}
