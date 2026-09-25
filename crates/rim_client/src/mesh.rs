//! What doesn't move, drawn from the GPU (DESIGN.md §8).
//!
//! Floors, items and fixtures are painted once per chunk into vertex
//! buffers that live on the GPU, and redrawn from there every frame: one
//! draw call per chunk and layer, and no vertices built or uploaded. A
//! chunk's buffers are rebuilt when its things revision moves (the map
//! bumps it for anything drawn there). Line widths and minimum sizes are in
//! screen points, so the zoom matters too: while it moves, chunks are drawn
//! scaled from the zoom they were built at, and rebuilt, a few a frame,
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

/// Geometry in screen points relative to the chunk's top-left corner, by
/// atlas page. Primitives sample page 0's white block.
pub struct Builder<'a> {
    atlas: &'a WorldAtlas,
    parts: Vec<(usize, Vec<Vert>, Vec<u16>)>,
}

impl<'a> Builder<'a> {
    fn new(atlas: &'a WorldAtlas) -> Self {
        Builder { atlas, parts: Vec::new() }
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
        let (v, i) = self.room(0, sides as usize + 2);
        let n = v.len() as u16;
        v.push(Vert { pos: [x, y], uv, color });
        for k in 0..=sides {
            let a = k as f32 / sides as f32 * std::f32::consts::TAU;
            v.push(Vert { pos: [x + r * a.cos(), y + r * a.sin()], uv, color });
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

struct Part {
    bindings: Bindings,
    indices: i32,
}

impl Chunk {
    fn free(&mut self, ctx: &mut dyn RenderingBackend) {
        for p in self.parts.iter_mut().flat_map(std::mem::take) {
            ctx.delete_buffer(p.bindings.vertex_buffers[0]);
            ctx.delete_buffer(p.bindings.index_buffer);
        }
    }
}

#[derive(Default)]
struct Chunk {
    /// The things revision and zoom the buffers were built at.
    built: Option<(u64, f32)>,
    /// Per layer: floors, items, fixtures.
    parts: [Vec<Part>; 3],
    /// Cells drawn each frame instead (worksites, animated looks), by layer.
    live: [Vec<IVec>; 3],
    /// Stack counts to label: cell and count.
    counts: Vec<(IVec, u32)>,
}

pub struct Meshes {
    pipeline: Option<Pipeline>,
    chunks: Vec<Chunk>,
    /// Chunks on screen this frame (from `prepare`).
    visible: Vec<usize>,
    /// The zoom last frame, and how many frames it has held.
    zoom: (f32, u32),
    /// Last frame, for the render bench and the profiler.
    pub calls: usize,
    pub indices: usize,
    pub rebuilt: usize,
    /// Time in `draw_layer` handing buffers to GL (µs).
    pub submit_us: f64,
}

impl Default for Meshes {
    fn default() -> Self {
        Meshes {
            pipeline: None,
            chunks: Vec::new(),
            visible: Vec::new(),
            zoom: (0.0, 0),
            calls: 0,
            indices: 0,
            rebuilt: 0,
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
uniform float scale;
uniform float flip;
void main() {
    vec2 p = (pos * scale + origin) / screen * 2.0 - 1.0;
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
    origin: [f32; 2],
    screen: [f32; 2],
    /// The zoom now over the zoom the chunk was built at.
    scale: f32,
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
                            UniformDesc::new("scale", UniformType::Float1),
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

    /// Paint chunk `c` into fresh buffers.
    #[allow(clippy::too_many_arguments)]
    fn build(&mut self, ctx: &mut dyn RenderingBackend, w: &World, atlas: &WorldAtlas, c: usize, z: f32, t: f32) {
        let IVec { x: x0, y: y0 } = w.map.chunk_origin(c);
        let chunk = &mut self.chunks[c];
        chunk.free(ctx);
        chunk.live = Default::default();
        chunk.counts.clear();
        for layer in 0..3 {
            let mut b = Builder::new(atlas);
            for y in y0..(y0 + CHUNK).min(w.map.h) {
                for x in x0..(x0 + CHUNK).min(w.map.w) {
                    let cell = IVec::new(x, y);
                    let i = w.map.idx(cell);
                    let Some(e) = w.map.layers_at(i)[layer] else { continue };
                    if live(w, e) {
                        chunk.live[layer].push(cell);
                        continue;
                    }
                    let at = ((x - x0) as f32 * z, (y - y0) as f32 * z);
                    if let Some(n) = draw::thing(&mut b, w, e, cell, at, z, t) {
                        chunk.counts.push((cell, n));
                    }
                }
            }
            chunk.parts[layer] = b
                .parts
                .into_iter()
                .filter(|(_, _, i)| !i.is_empty())
                .map(|(page, v, i)| Part {
                    indices: i.len() as i32,
                    bindings: Bindings {
                        vertex_buffers: vec![ctx.new_buffer(
                            BufferType::VertexBuffer,
                            BufferUsage::Immutable,
                            BufferSource::slice(&v),
                        )],
                        index_buffer: ctx.new_buffer(
                            BufferType::IndexBuffer,
                            BufferUsage::Immutable,
                            BufferSource::slice(&i),
                        ),
                        images: vec![atlas.pages[page].raw_miniquad_id()],
                    },
                })
                .collect();
        }
        chunk.built = Some((w.map.things_rev(c), z));
        self.rebuilt += 1;
    }

    /// Find this frame's visible chunks and rebuild the stale ones.
    pub fn prepare(&mut self, w: &World, atlas: &WorldAtlas, cam: &Cam, t: f32) {
        // SAFETY: macroquad's context outlives the frame, and nothing else
        // holds it while the world draws.
        let gl = unsafe { get_internal_gl() };
        let ctx = gl.quad_context;
        let (cx, cy) = w.map.chunks();
        if self.chunks.len() != (cx * cy) as usize {
            for ch in &mut self.chunks {
                ch.free(ctx);
            }
            self.chunks = (0..cx * cy).map(|_| Chunk::default()).collect();
        }
        let (wx0, wy0) = cam.to_world(0.0, 0.0);
        let (wx1, wy1) = cam.to_world(screen_width(), screen_height());
        let span = |a: f32, b: f32, n: i32| {
            ((a / CHUNK as f32).floor().max(0.0) as i32, ((b / CHUNK as f32).floor() as i32).min(n - 1))
        };
        let ((c0x, c1x), (c0y, c1y)) = (span(wx0, wx1, cx), span(wy0, wy1, cy));
        self.visible = (c0y..=c1y).flat_map(|y| (c0x..=c1x).map(move |x| (y * cx + x) as usize)).collect();

        if self.pipeline.is_none() {
            self.pipeline = Some(Self::pipeline(ctx));
        }
        self.rebuilt = 0;
        (self.calls, self.indices, self.submit_us) = (0, 0, 0.0);
        self.zoom = if self.zoom.0 == cam.zoom { (cam.zoom, self.zoom.1 + 1) } else { (cam.zoom, 0) };
        let settled = self.zoom.1 >= SETTLE_FRAMES;
        let start = std::time::Instant::now();
        for k in 0..self.visible.len() {
            let c = self.visible[k];
            let stale = match self.chunks[c].built {
                None => true,
                Some((r, _)) if r != w.map.things_rev(c) => true,
                Some((_, z)) if z == cam.zoom => false,
                Some((_, z)) => {
                    let far = (cam.zoom / z).max(z / cam.zoom) > MAX_SCALE;
                    (settled || far) && start.elapsed().as_secs_f64() * 1e6 < ZOOM_BUDGET_US
                }
            };
            if stale {
                self.build(ctx, w, atlas, c, cam.zoom, t);
            }
        }
    }

    /// Draw one layer (floors, items, fixtures) of every visible chunk from
    /// its buffers. Whatever macroquad has batched so far goes first, so
    /// the layers below stay below.
    pub fn draw_layer(&mut self, w: &World, cam: &Cam, layer: usize, target: Option<RenderPass>) {
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
        for &c in &self.visible {
            let o = w.map.chunk_origin(c);
            let origin = cam.to_screen(o.x as f32, o.y as f32);
            let scale = self.chunks[c].built.map_or(1.0, |(_, z)| cam.zoom / z);
            for p in &self.chunks[c].parts[layer] {
                ctx.apply_bindings(&p.bindings);
                ctx.apply_uniforms(UniformsSource::table(&Uniforms {
                    origin: [origin.0, origin.1],
                    screen,
                    scale,
                    flip,
                }));
                ctx.draw(0, p.indices, 1);
                self.calls += 1;
                self.indices += p.indices as usize;
            }
        }
        ctx.end_render_pass();
        self.submit_us += start.elapsed().as_secs_f64() * 1e6;
    }

    /// Cells of `layer` in the visible chunks to draw live this frame.
    pub fn live(&self, layer: usize) -> impl Iterator<Item = IVec> + '_ {
        self.visible.iter().flat_map(move |&c| self.chunks[c].live[layer].iter().copied())
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
