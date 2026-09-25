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
//! Plans and animated looks change every frame, so they stay out of the
//! buffers and are drawn by `draw::things` each frame.

use crate::draw::{self, Sink};
use crate::Cam;
use macroquad::miniquad::*;
use macroquad::prelude::{get_internal_gl, screen_height, screen_width, Color};
use rim_sim::map::CHUNK;
use rim_sim::world::{Blueprint, Thing, World};
use rim_sim::IVec;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vert {
    pos: [f32; 2],
    color: [u8; 4],
}

/// u16 indices: a buffer holds at most this many vertices.
const MAX_VERTS: usize = u16::MAX as usize;

/// Geometry in screen points relative to the chunk's top-left corner.
#[derive(Default)]
pub struct Builder {
    parts: Vec<(Vec<Vert>, Vec<u16>)>,
}

impl Builder {
    fn room(&mut self, verts: usize) -> &mut (Vec<Vert>, Vec<u16>) {
        if self.parts.last().is_none_or(|p| p.0.len() + verts > MAX_VERTS) {
            self.parts.push((Vec::new(), Vec::new()));
        }
        self.parts.last_mut().expect("just pushed")
    }

    fn quad(&mut self, p: [[f32; 2]; 4], c: Color) {
        let color = [(c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8, (c.a * 255.0) as u8];
        let (v, i) = self.room(4);
        let n = v.len() as u16;
        v.extend(p.map(|pos| Vert { pos, color }));
        i.extend([n, n + 1, n + 2, n, n + 2, n + 3]);
    }
}

impl Sink for Builder {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        self.quad([[x, y], [x + w, y], [x + w, y + h], [x, y + h]], c);
    }

    /// Like macroquad's `draw_poly`: a fan of `sides` triangles.
    fn poly(&mut self, x: f32, y: f32, sides: u8, r: f32, c: Color) {
        let color = [(c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8, (c.a * 255.0) as u8];
        let (v, i) = self.room(sides as usize + 2);
        let n = v.len() as u16;
        v.push(Vert { pos: [x, y], color });
        for k in 0..=sides {
            let a = k as f32 / sides as f32 * std::f32::consts::TAU;
            v.push(Vert { pos: [x + r * a.cos(), y + r * a.sin()], color });
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
        self.quad([[x0 + tx, y0 + ty], [x0 - tx, y0 - ty], [x1 - tx, y1 - ty], [x1 + tx, y1 + ty]], c);
    }
}

struct Part {
    bindings: Bindings,
    indices: i32,
}

#[derive(Default)]
struct Chunk {
    /// The things revision and zoom the buffers were built at.
    built: Option<(u64, f32)>,
    /// Per layer: floors, items, fixtures.
    parts: [Vec<Part>; 3],
    /// Cells drawn each frame instead (plans, animated looks), by layer.
    live: [Vec<IVec>; 3],
    /// Stack counts to label: cell and count.
    counts: Vec<(IVec, u32)>,
}

pub struct Meshes {
    pipeline: Option<Pipeline>,
    chunks: Vec<Chunk>,
    /// The zoom last frame, and how many frames it has held.
    zoom: (f32, u32),
    /// Last frame, for the render bench and the profiler.
    pub calls: usize,
    pub indices: usize,
    pub rebuilt: usize,
}

impl Default for Meshes {
    fn default() -> Self {
        Meshes { pipeline: None, chunks: Vec::new(), zoom: (0.0, 0), calls: 0, indices: 0, rebuilt: 0 }
    }
}

const VERTEX: &str = "#version 100
attribute vec2 pos;
attribute vec4 color0;
varying lowp vec4 color;
uniform vec2 origin;
uniform vec2 screen;
uniform float scale;
void main() {
    vec2 p = (pos * scale + origin) / screen * 2.0 - 1.0;
    gl_Position = vec4(p.x, -p.y, 0.0, 1.0);
    color = color0 / 255.0;
}";

const FRAGMENT: &str = "#version 100
varying lowp vec4 color;
void main() {
    gl_FragColor = color;
}";

#[repr(C)]
struct Uniforms {
    origin: [f32; 2],
    screen: [f32; 2],
    /// The zoom now over the zoom the chunk was built at.
    scale: f32,
}

/// Frames the zoom must hold before stale chunks are rebuilt at it.
const SETTLE_FRAMES: u32 = 8;
/// Past this ratio of zooms a scaled chunk no longer passes (lines twice
/// as thick), so it is rebuilt even mid-gesture.
const MAX_SCALE: f32 = 2.0;
/// Rebuilding for the zoom stops for the frame after this long; the rest
/// wait, drawn scaled. Content changes always rebuild.
const ZOOM_BUDGET_US: f64 = 1500.0;

/// Should this thing be drawn each frame rather than cached?
fn live(w: &World, e: rim_sim::hecs::Entity) -> bool {
    if w.ecs.get::<&Blueprint>(e).is_ok() {
        return true;
    }
    w.ecs.get::<&Thing>(e).is_ok_and(|t| w.defs.thing(t.def).look_r.animated())
}

impl Meshes {
    fn pipeline(ctx: &mut dyn RenderingBackend) -> Pipeline {
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
                ShaderMeta {
                    images: vec![],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("origin", UniformType::Float2),
                            UniformDesc::new("screen", UniformType::Float2),
                            UniformDesc::new("scale", UniformType::Float1),
                        ],
                    },
                },
            )
            // The shader is GLSL 100, the lowest every backend compiles.
            .expect("the chunk shader compiles");
        ctx.new_pipeline(
            &[BufferLayout::default()],
            &[VertexAttribute::new("pos", VertexFormat::Float2), VertexAttribute::new("color0", VertexFormat::Byte4)],
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
    fn build(&mut self, ctx: &mut dyn RenderingBackend, w: &World, c: usize, z: f32, t: f32) {
        let (cx, _) = w.map.chunks();
        let (x0, y0) = ((c as i32 % cx) * CHUNK, (c as i32 / cx) * CHUNK);
        let chunk = &mut self.chunks[c];
        for p in chunk.parts.iter_mut().flatten() {
            ctx.delete_buffer(p.bindings.vertex_buffers[0]);
            ctx.delete_buffer(p.bindings.index_buffer);
        }
        chunk.live = Default::default();
        chunk.counts.clear();
        for layer in 0..3 {
            let mut b = Builder::default();
            for y in y0..(y0 + CHUNK).min(w.map.h) {
                for x in x0..(x0 + CHUNK).min(w.map.w) {
                    let cell = IVec::new(x, y);
                    let i = w.map.idx(cell);
                    let Some(e) = [w.map.floor[i], w.map.item[i], w.map.fixture[i]][layer] else { continue };
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
                .filter(|(_, i)| !i.is_empty())
                .map(|(v, i)| Part {
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
                        images: vec![],
                    },
                })
                .collect();
        }
        chunk.built = Some((w.map.things_rev(c), z));
        self.rebuilt += 1;
    }

    /// Draw the cached layers of every visible chunk, rebuilding stale
    /// ones first. Returns, per layer, the cells to draw live this frame,
    /// and the stack counts to label.
    pub fn draw(&mut self, w: &World, cam: &Cam, t: f32) -> ([Vec<IVec>; 3], Vec<(IVec, u32)>) {
        let (cx, cy) = w.map.chunks();
        if self.chunks.len() != (cx * cy) as usize {
            self.chunks = (0..cx * cy).map(|_| Chunk::default()).collect();
        }
        let (sw, sh) = (screen_width(), screen_height());
        let (wx0, wy0) = cam.to_world(0.0, 0.0);
        let (wx1, wy1) = cam.to_world(sw, sh);
        let span = |a: f32, b: f32, n: i32| {
            ((a / CHUNK as f32).floor().max(0.0) as i32, ((b / CHUNK as f32).floor() as i32).min(n - 1))
        };
        let ((c0x, c1x), (c0y, c1y)) = (span(wx0, wx1, cx), span(wy0, wy1, cy));
        let visible: Vec<usize> = (c0y..=c1y).flat_map(|y| (c0x..=c1x).map(move |x| (y * cx + x) as usize)).collect();

        // SAFETY: macroquad's context outlives the frame, and nothing else holds
        // it while the world draws.
        let mut gl = unsafe { get_internal_gl() };
        // What macroquad has batched so far (the ground) goes first.
        gl.flush();
        let ctx = gl.quad_context;
        if self.pipeline.is_none() {
            self.pipeline = Some(Self::pipeline(ctx));
        }
        self.rebuilt = 0;
        self.zoom = if self.zoom.0 == cam.zoom { (cam.zoom, self.zoom.1 + 1) } else { (cam.zoom, 0) };
        let settled = self.zoom.1 >= SETTLE_FRAMES;
        let start = std::time::Instant::now();
        for &c in &visible {
            let rev = w.map.things_rev(c);
            let stale = match self.chunks[c].built {
                None => true,
                Some((r, _)) if r != rev => true,
                Some((_, z)) if z == cam.zoom => false,
                Some((_, z)) => {
                    let ratio = (cam.zoom / z).max(z / cam.zoom);
                    let in_budget = start.elapsed().as_secs_f64() * 1e6 < ZOOM_BUDGET_US;
                    ratio > MAX_SCALE || (settled && in_budget)
                }
            };
            if stale {
                self.build(ctx, w, c, cam.zoom, t);
            }
        }

        let (mut calls, mut indices) = (0, 0);
        ctx.begin_default_pass(PassAction::Nothing);
        ctx.apply_pipeline(self.pipeline.as_ref().expect("made above"));
        for layer in 0..3 {
            for &c in &visible {
                let (x0, y0) = ((c as i32 % cx) * CHUNK, (c as i32 / cx) * CHUNK);
                let origin = cam.to_screen(x0 as f32, y0 as f32);
                let scale = self.chunks[c].built.map_or(1.0, |(_, z)| cam.zoom / z);
                for p in &self.chunks[c].parts[layer] {
                    ctx.apply_bindings(&p.bindings);
                    ctx.apply_uniforms(UniformsSource::table(&Uniforms {
                        origin: [origin.0, origin.1],
                        screen: [sw, sh],
                        scale,
                    }));
                    ctx.draw(0, p.indices, 1);
                    calls += 1;
                    indices += p.indices as usize;
                }
            }
        }
        ctx.end_render_pass();
        (self.calls, self.indices) = (calls, indices);

        let mut live: [Vec<IVec>; 3] = Default::default();
        let mut counts = Vec::new();
        for &c in &visible {
            let ch = &self.chunks[c];
            for (l, cells) in ch.live.iter().enumerate() {
                live[l].extend(cells);
            }
            counts.extend(&ch.counts);
        }
        (live, counts)
    }
}
