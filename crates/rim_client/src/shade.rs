//! Shadows as shapes (DESIGN.md §6e, f05c5fa1). Each caster is an outline
//! with a height, built once with its block of cells; a vertex shader
//! pushes the outline away from the sky body, so a moving sun rebuilds
//! nothing.
//!
//! Masses (walls, rock, roofs) are outlined by marching squares over the
//! grid of cell centres: straight runs lie on cell edges, and a staircase
//! of cells becomes one diagonal, with no steps. A tree is a thin trunk and
//! a crown at the end of it. Every shape is swept from its foot (`along` 0)
//! to its tip (`along` 1), dark at the foot and lighter out.
//!
//! The shapes live on the GPU as the world's chunks do (`mesh`), nothing
//! built or uploaded while nothing changes, but the whole level in as few
//! buffers as fit: a draw each, about three on the whole bench map.

use macroquad::miniquad::*;

/// What stands in a cell, as a shadow sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Caster {
    None,
    /// A wall, rock, a roof: its height, in storeys.
    Mass(f32),
    /// A tree: its crown's height.
    Crown(f32),
    /// Rock under rock (`COVERED`): mass, which takes no shadow, but with
    /// no sky over it to cast in.
    Under(f32),
}

/// A cell's caster, from its occluder texel (`occluders::texel`): what
/// stands a height tall, a canopy's crown, rock under rock, or nothing.
pub fn caster(t: [u8; 4]) -> Caster {
    let height = (t[0] >> 2) as f32 / 63.0 * crate::occluders::MAX_HEIGHT as f32;
    if height <= 0.0 {
        Caster::None
    } else if (t[0] & 3) == crate::occluders::COVERED {
        Caster::Under(height)
    } else if t[1] == 0 && t[2] > 0 && t[2] < 250 {
        Caster::Crown(height)
    } else {
        Caster::Mass(height)
    }
}

/// A shadow vertex. `pos` in cells, the height of what casts it, how far
/// along its sweep it is, and how dark the shadow is there. `edge` is the
/// outward normal of the mass edge it sweeps, 0 on a cap. A tree is one
/// quad: its vertices stand at its middle, `dark` -1, `edge.x` -1 or 1
/// across it, and the shaders stretch it along its shadow and cut the shape
/// out.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadeVertex {
    pub pos: [f32; 2],
    pub height: f32,
    pub along: f32,
    pub dark: f32,
    pub edge: [f32; 2],
}

/// How dark a shadow is at its tip, against 1 at its foot (the shaders' TIP).
pub const TIP: f32 = 0.3;

/// The shapes for the cells `[x0, x1) × [y0, y1)`. `at` answers for any
/// cell, the neighbours round the edge included. A block of four cell
/// centres belongs to the cells it has its lower-right centre in, so
/// neighbouring chunks don't draw the same block twice.
///
/// Outside a mass, its shadow is its boundary swept away from the body, so
/// a mass is drawn as its boundary's edges, each a quad, and where it steps
/// up, the step's. Its inside takes no shadow. A cap at the foot fills what
/// the outline takes of the cells beside it, where it cuts their corners.
pub fn build(x0: i32, y0: i32, x1: i32, y1: i32, at: &impl Fn(i32, i32) -> Caster) -> (Vec<ShadeVertex>, Vec<u32>) {
    let (mut verts, mut idx) = (Vec::new(), Vec::new());
    let mass = |x: i32, y: i32| match at(x, y) {
        Caster::Mass(h) => Some(h),
        _ => None,
    };
    // Rock under rock takes no shadow either, so a mass's rim beside it
    // needn't cast onto it.
    let solid = |x: i32, y: i32| match at(x, y) {
        Caster::Mass(h) | Caster::Under(h) => Some(h),
        _ => None,
    };
    for y in y0..y1 {
        for x in x0..x1 {
            // The block of centres (x-1, y-1) to (x, y), clockwise from the top left.
            let corners = [(x - 1, y - 1), (x, y - 1), (x, y), (x - 1, y)];
            let h = corners.map(|(cx, cy)| mass(cx, cy));
            if h.iter().all(Option::is_none) || covered(x, y, &solid) {
                continue;
            }
            let centres = corners.map(|(cx, cy)| [cx as f32 + 0.5, cy as f32 + 0.5]);
            let points = outline(centres, h);
            if h.iter().any(Option::is_none) {
                cap(&mut verts, &mut idx, &points);
            }
            edges(&mut verts, &mut idx, &points);
            // A step up inside the mass: the taller part's edge casts past
            // the lower, at the taller height.
            let tallest = h.iter().flatten().fold(0.0f32, |t, &v| t.max(v));
            if h.iter().flatten().any(|&v| v < tallest) {
                let top = h.map(|v| v.filter(|&v| v >= tallest));
                edges(&mut verts, &mut idx, &outline(centres, top));
            }
        }
    }
    for y in y0..y1 {
        for x in x0..x1 {
            let Caster::Crown(height) = at(x, y) else { continue };
            let pos = [x as f32 + 0.5, y as f32 + 0.5];
            let base = verts.len() as u32;
            for (along, side) in [(0.0, -1.0), (1.0, -1.0), (1.0, 1.0), (0.0, 1.0)] {
                verts.push(ShadeVertex { pos, height, along, dark: -1.0, edge: [side, 0.0] });
            }
            idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
    (verts, idx)
}

/// A block's outline, clockwise: each point, the height it stands at, and
/// whether it's where the mass ends between two centres (a cell edge's
/// middle) rather than a centre.
fn outline(centres: [[f32; 2]; 4], h: [Option<f32>; 4]) -> Vec<([f32; 2], f32, bool)> {
    let mut points = Vec::with_capacity(8);
    for i in 0..4 {
        let j = (i + 1) % 4;
        if let Some(hi) = h[i] {
            points.push((centres[i], hi, false));
        }
        if h[i].is_some() != h[j].is_some() {
            let (a, b) = (centres[i], centres[j]);
            points.push(([(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0], h[i].or(h[j]).unwrap_or(0.0), true));
        }
    }
    points
}

/// The outline at the foot, a fan of triangles.
fn cap(verts: &mut Vec<ShadeVertex>, idx: &mut Vec<u32>, points: &[([f32; 2], f32, bool)]) {
    let (n, base) = (points.len() as u32, verts.len() as u32);
    if n < 3 {
        return;
    }
    verts.extend(points.iter().map(|&(pos, height, _)| ShadeVertex {
        pos,
        height,
        along: 0.0,
        dark: 1.0,
        edge: [0.0; 2],
    }));
    for i in 1..n - 1 {
        idx.extend([base, base + i, base + i + 1]);
    }
}

/// The outline's boundary edges, those from one cell edge's middle to the
/// next, each swept from the foot to the tip: a quad, dark to light.
fn edges(verts: &mut Vec<ShadeVertex>, idx: &mut Vec<u32>, points: &[([f32; 2], f32, bool)]) {
    let n = points.len();
    for i in 0..n {
        let (p, q) = (points[i], points[(i + 1) % n]);
        if !(p.2 && q.2) {
            continue;
        }
        // Clockwise with the mass on the right, so out is to the left.
        let (dx, dy) = (q.0[0] - p.0[0], q.0[1] - p.0[1]);
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let edge = [dy / len, -dx / len];
        let base = verts.len() as u32;
        for (point, along, dark) in [(p, 0.0, 1.0), (q, 0.0, 1.0), (q, 1.0, TIP), (p, 1.0, TIP)] {
            verts.push(ShadeVertex { pos: point.0, height: point.1, along, dark, edge });
        }
        idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

/// Whether the block at (x, y) casts nothing its neighbours don't: its
/// cells and the twelve around them are all solid (`build`), none lower
/// than its tallest. A shadow leaving it crosses a neighbour at least as
/// tall, whose own shadow reaches as far on from there; inside, it falls on
/// mass, which takes none. So only a mass's rim and its steps up are drawn.
/// Rock under rock casts nothing itself: past it, the rock on its far side
/// does, as tall as a mountain's rim stands all round.
fn covered(x: i32, y: i32, solid: &impl Fn(i32, i32) -> Option<f32>) -> bool {
    let mut own = (y - 1..=y).flat_map(|cy| (x - 1..=x).map(move |cx| (cx, cy))).map(|(cx, cy)| solid(cx, cy));
    let Some(tallest) = own.try_fold(0.0f32, |t, h| Some(t.max(h?))) else { return false };
    (y - 2..=y + 1).all(|cy| (x - 2..=x + 1).all(|cx| solid(cx, cy).is_some_and(|h| h >= tallest)))
}

/// Cells a side of a block, the shapes built at a time: the most a block
/// can hold stays within one u16-indexed buffer.
const BLOCK: i32 = 32;
const MAX_VERTS: usize = u16::MAX as usize;

/// A mass edge's vertex stands at its foot in cells, and is pushed away
/// from the sky body by how far along its sweep it is, times its caster's
/// height and the body's cot(altitude) (`push`'s length), up to `longest`;
/// an edge facing the body isn't pushed at all. A tree's
/// quad runs from a crown's width behind its middle to a crown's width past
/// its shadow's tip, a crown's width to each side; the fragment shader keeps
/// the trunk's shadow, a thin strip out to half way or a cell, and the
/// crown's, a disc swept over the far half. Darkness goes to depth, so the darkest shadow
/// wins each pixel: a max, where GLES2 blends have none.
const VERTEX: &str = "#version 100
precision highp float;
attribute vec2 pos;
attribute vec3 sweep;
attribute vec2 edge;
varying float dark;
varying vec3 tree;
uniform vec2 centre;
uniform vec2 scale;
uniform vec2 push;
uniform float longest;
const float CROWN = 0.42;
const float FOOT = 0.8;
const float TIP = 0.3;
void main() {
    float cot = length(push);
    vec2 away = cot > 0.0 ? push / cot : vec2(1.0, 0.0);
    vec2 p;
    if (sweep.z >= 0.0) {
        // An edge facing the body sweeps over its own mass, which takes no
        // shadow: it stays a line, and costs nothing.
        float t = dot(edge, away) < 0.0 ? 0.0 : sweep.y;
        p = pos + away * min(sweep.x * cot, longest) * t;
        dark = sweep.z;
        tree = vec3(0.0, 0.0, -1.0);
    } else {
        float side = edge.x;
        float len = min(sweep.x * cot, longest);
        float s = mix(-CROWN, len + CROWN, sweep.y);
        p = pos + away * s + vec2(-away.y, away.x) * side * CROWN;
        // Linear along the quad, so depth keeps it: foot to tip over the
        // shadow, a cell at least.
        dark = mix(FOOT, TIP, s / max(len, 1.0));
        tree = vec3(s, side * CROWN, len);
    }
    gl_Position = vec4((p - centre) * scale, 1.0 - dark, 1.0);
}";

const FRAGMENT: &str = "#version 100
precision mediump float;
varying float dark;
varying vec3 tree;
const float TRUNK = 0.07;
const float CROWN = 0.42;
void main() {
    // A tree's quad: along its shadow, across it, and its length.
    if (tree.z >= 0.0) {
        float s = tree.x;
        float len = tree.z;
        float c = clamp(s, len * 0.45, len);
        bool crown = length(vec2(s - c, tree.y)) < CROWN;
        // The trunk's shadow reads as a trunk near the tree; at a low sun,
        // stretched half way out, it would read as a stick.
        bool trunk = abs(tree.y) < TRUNK && s > 0.0 && s < min(len * 0.55, 1.0);
        if (!crown && !trunk) {
            discard;
        }
    }
    gl_FragColor = vec4(clamp(dark, 0.0, 1.0), 0.0, 0.0, 1.0);
}";

#[repr(C)]
struct Uniforms {
    /// The cell at the mask's centre, and clip units a cell.
    centre: [f32; 2],
    scale: [f32; 2],
    push: [f32; 2],
    longest: f32,
}

struct Part {
    /// Its buffers; none while its blocks hold no shapes.
    buffers: Option<Bindings>,
    /// Where its shapes stand, in cells: x0, y0, x1, y1.
    bounds: [f32; 4],
    /// The blocks packed in it, in order, each with its run of indices:
    /// block, first index and count.
    runs: Vec<(usize, i32, i32)>,
}

impl Default for Part {
    fn default() -> Part {
        Part { buffers: None, bounds: [f32::MAX, f32::MAX, f32::MIN, f32::MIN], runs: Vec::new() }
    }
}

/// One level's shadow shapes: built a block at a time on the CPU, where
/// they change, and on the GPU the whole level in as few buffers as u16
/// indices allow, a draw each: shapes sample no texture, so nothing else
/// splits them.
#[derive(Default)]
pub struct Shapes {
    /// Each block's shapes (row major), indexed from its first vertex.
    blocks: Vec<(Vec<ShadeVertex>, Vec<u16>)>,
    parts: Vec<Part>,
    /// The part each block is packed in.
    part_of: Vec<usize>,
    /// Blocks across the map.
    across: i32,
    /// The occluder version they were built from.
    of: Option<u64>,
}

/// The pipeline the shapes draw with: into a mask with depth, no blend.
pub fn pipeline(ctx: &mut dyn RenderingBackend) -> Result<Pipeline, ShaderError> {
    let shader = ctx.new_shader(
        ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
        ShaderMeta {
            images: vec![],
            uniforms: UniformBlockLayout {
                uniforms: vec![
                    UniformDesc::new("centre", UniformType::Float2),
                    UniformDesc::new("scale", UniformType::Float2),
                    UniformDesc::new("push", UniformType::Float2),
                    UniformDesc::new("longest", UniformType::Float1),
                ],
            },
        },
    )?;
    Ok(ctx.new_pipeline(
        &[BufferLayout::default()],
        &[
            VertexAttribute::new("pos", VertexFormat::Float2),
            VertexAttribute::new("sweep", VertexFormat::Float3),
            VertexAttribute::new("edge", VertexFormat::Float2),
        ],
        shader,
        PipelineParams { depth_test: Comparison::Less, depth_write: true, color_blend: None, ..Default::default() },
    ))
}

impl Shapes {
    /// Bring the shapes up to occluder `version` on a `w` by `h` map: every
    /// block the first time or when `whole`, else those within reach of a
    /// `changed` box (x, y, w, h), and then the buffers. Blocks built, for
    /// the profiler.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        (w, h): (i32, i32),
        version: u64,
        whole: bool,
        changed: &[(i32, i32, i32, i32)],
        at: impl Fn(i32, i32) -> Caster,
    ) -> usize {
        if self.of == Some(version) {
            return 0;
        }
        let (bw, bh) = ((w + BLOCK - 1) / BLOCK, (h + BLOCK - 1) / BLOCK);
        let whole = whole || self.of.is_none() || self.blocks.len() != (bw * bh) as usize;
        if self.blocks.len() != (bw * bh) as usize {
            self.blocks = (0..bw * bh).map(|_| Default::default()).collect();
        }
        self.across = bw;
        // A shape reads cells two beyond its own (`build`).
        let near = |&(x, y, cw, ch): &(i32, i32, i32, i32), x0: i32, y0: i32| {
            x < x0 + BLOCK + 2 && x + cw > x0 - 2 && y < y0 + BLOCK + 2 && y + ch > y0 - 2
        };
        let mut built = Vec::new();
        for by in 0..bh {
            for bx in 0..bw {
                let (x0, y0) = (bx * BLOCK, by * BLOCK);
                if !whole && !changed.iter().any(|b| near(b, x0, y0)) {
                    continue;
                }
                let (v, i) = build(x0, y0, (x0 + BLOCK).min(w), (y0 + BLOCK).min(h), &at);
                let b = (by * bw + bx) as usize;
                self.blocks[b] = (v, i.iter().map(|&k| k as u16).collect());
                built.push(b);
            }
        }
        self.of = Some(version);
        if whole || self.part_of.len() != self.blocks.len() {
            for p in self.parts.drain(..) {
                free(ctx, p);
            }
            self.part_of = vec![0; self.blocks.len()];
            self.pack(ctx, (0..self.blocks.len()).collect());
        } else {
            // An edit re-packs only the parts it touched, not the level.
            let mut touched: Vec<usize> = built.iter().map(|&b| self.part_of[b]).collect();
            touched.sort_unstable();
            touched.dedup();
            for &k in touched.iter().rev() {
                let p = self.parts.remove(k);
                for b in &mut self.part_of {
                    *b -= (*b > k) as usize;
                }
                let blocks = p.runs.iter().map(|r| r.0).collect();
                free(ctx, p);
                self.pack(ctx, blocks);
            }
        }
        built.len()
    }

    /// Pack `blocks`, in order, into as few new parts as fit, each with the
    /// box its shapes stand in.
    fn pack(&mut self, ctx: &mut dyn RenderingBackend, blocks: Vec<usize>) {
        let mut part = Part::default();
        let (mut verts, mut idx): (Vec<ShadeVertex>, Vec<u16>) = (Vec::new(), Vec::new());
        for b in blocks {
            if verts.len() + self.blocks[b].0.len() > MAX_VERTS {
                self.finish(ctx, std::mem::take(&mut part), &verts, &idx);
                (verts, idx) = (Vec::new(), Vec::new());
            }
            let (v, i) = &self.blocks[b];
            let base = verts.len() as u16;
            part.runs.push((b, idx.len() as i32, i.len() as i32));
            idx.extend(i.iter().map(|&k| k + base));
            for s in v {
                let [a, c, d, e] = part.bounds;
                part.bounds = [a.min(s.pos[0]), c.min(s.pos[1]), d.max(s.pos[0]), e.max(s.pos[1])];
            }
            verts.extend_from_slice(v);
        }
        self.finish(ctx, part, &verts, &idx);
    }

    /// Upload a packed part and note its blocks as in it; an empty one keeps
    /// its blocks, drawing nothing, so a shape that comes there later has a
    /// part to go to.
    fn finish(&mut self, ctx: &mut dyn RenderingBackend, mut part: Part, verts: &[ShadeVertex], idx: &[u16]) {
        if part.runs.is_empty() {
            return;
        }
        if !idx.is_empty() {
            part.buffers = Some(upload(ctx, verts, idx));
        }
        for &(b, ..) in &part.runs {
            self.part_of[b] = self.parts.len();
        }
        self.parts.push(part);
    }

    /// Draw the blocks that could shadow cells `[x0, x1) × [y0, y1)` into
    /// the pass begun: those within `longest` cells of it, blocks next to
    /// each other in a buffer in one draw. `centre` is the cell at the
    /// mask's centre, `scale` clip units a cell. Draw calls made, and
    /// indices drawn.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        ctx: &mut dyn RenderingBackend,
        pipeline: &Pipeline,
        (x0, y0, x1, y1): (f32, f32, f32, f32),
        centre: [f32; 2],
        scale: [f32; 2],
        push: [f32; 2],
        longest: f32,
    ) -> (usize, usize) {
        ctx.apply_pipeline(pipeline);
        ctx.apply_uniforms(UniformsSource::table(&Uniforms { centre, scale, push, longest }));
        // A block's shapes reach a cell past it, and its shadows `longest`.
        let reach = longest + 1.0;
        let seen = |b: usize| {
            let (bx, by) = ((b as i32 % self.across * BLOCK) as f32, (b as i32 / self.across * BLOCK) as f32);
            bx - reach < x1 && bx + BLOCK as f32 + reach > x0 && by - reach < y1 && by + BLOCK as f32 + reach > y0
        };
        let (mut calls, mut indices) = (0, 0);
        for p in &self.parts {
            let [a, b, c, d] = p.bounds;
            let Some(bindings) = &p.buffers else { continue };
            if c + longest < x0 || a - longest > x1 || d + longest < y0 || b - longest > y1 {
                continue;
            }
            ctx.apply_bindings(bindings);
            // Runs of blocks in view, joined where they sit end to end.
            let mut run: Option<(i32, i32)> = None;
            for &(_, first, count) in p.runs.iter().filter(|r| r.2 > 0 && seen(r.0)) {
                run = match run {
                    Some((f, n)) if f + n == first => Some((f, n + count)),
                    Some((f, n)) => {
                        ctx.draw(f, n, 1);
                        (calls, indices) = (calls + 1, indices + n as usize);
                        Some((first, count))
                    }
                    None => Some((first, count)),
                };
            }
            if let Some((f, n)) = run {
                ctx.draw(f, n, 1);
                (calls, indices) = (calls + 1, indices + n as usize);
            }
        }
        (calls, indices)
    }

    /// Give the buffers back to GL.
    pub fn free(&mut self, ctx: &mut dyn RenderingBackend) {
        for p in self.parts.drain(..) {
            free(ctx, p);
        }
        self.blocks.clear();
        self.part_of.clear();
        self.of = None;
    }
}

fn upload(ctx: &mut dyn RenderingBackend, verts: &[ShadeVertex], idx: &[u16]) -> Bindings {
    Bindings {
        vertex_buffers: vec![ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(verts),
        )],
        index_buffer: ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(idx)),
        images: vec![],
    }
}

fn free(ctx: &mut dyn RenderingBackend, p: Part) {
    if let Some(b) = p.buffers {
        ctx.delete_buffer(b.vertex_buffers[0]);
        ctx.delete_buffer(b.index_buffer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shapes of a small map drawn as text: `#` mass a storey tall, `T`
    /// a tree, anything else open.
    fn shapes(rows: &[&str]) -> (Vec<ShadeVertex>, Vec<u32>) {
        let at = |x: i32, y: i32| match rows.get(y as usize).and_then(|r| r.as_bytes().get(x as usize)) {
            Some(b'#') if x >= 0 && y >= 0 => Caster::Mass(1.0),
            Some(b'T') if x >= 0 && y >= 0 => Caster::Crown(2.0),
            _ => Caster::None,
        };
        build(0, 0, rows[0].len() as i32 + 1, rows.len() as i32 + 1, &at)
    }

    #[test]
    fn a_mass_outline_follows_cell_edges_and_cuts_its_steps_diagonal() {
        // One cell: its outline is its own square, corner to corner on its edges.
        let (v, i) = shapes(&["...", ".#.", "..."]);
        let feet: Vec<[f32; 2]> = v.iter().filter(|v| v.along == 0.0).map(|v| v.pos).collect();
        let (lo, hi) = feet.iter().fold(([9.0f32; 2], [-9.0f32; 2]), |(lo, hi), p| {
            ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])])
        });
        assert_eq!((lo, hi), ([1.0, 1.0], [2.0, 2.0]), "a lone cell's outline stays within its edges");
        for mid in [[1.5, 1.0], [2.0, 1.5], [1.5, 2.0], [1.0, 1.5]] {
            assert!(feet.contains(&mid), "through its edges' middles: {feet:?}");
        }
        assert!(!i.is_empty() && i.iter().all(|&k| (k as usize) < v.len()));
        // A staircase: no outline turns at a cell's corner, where a step would,
        // so it runs diagonal from centre to centre.
        let (v, _) = shapes(&["#...", "##..", ".##.", "..##"]);
        let corner = |p: &[f32; 2]| p[0].fract() == 0.0 && p[1].fract() == 0.0;
        assert!(v.iter().all(|s| !corner(&s.pos)), "no step corners");
        let feet: Vec<[f32; 2]> = v.iter().filter(|v| v.along == 0.0).map(|v| v.pos).collect();
        assert!(feet.contains(&[1.5, 1.5]) && feet.contains(&[2.5, 2.5]), "through the cell centres");
    }

    #[test]
    fn a_mass_sweeps_dark_to_light_and_a_tree_is_one_quad() {
        let (v, i) = shapes(&["...", ".#.", "..T"]);
        let mass: Vec<_> = v.iter().filter(|s| s.dark >= 0.0).collect();
        for s in &mass {
            assert!((0.0..=1.0).contains(&s.along) && s.dark > 0.0 && s.dark <= 1.0, "{s:?}");
        }
        assert!(
            mass.iter().any(|s| s.along == 0.0 && s.dark == 1.0)
                && mass.iter().any(|s| s.along == 1.0 && s.dark == TIP)
        );
        // Four corners at the tree's middle, foot and tip, either side; the
        // shaders stretch it and cut the trunk and crown out.
        let tree: Vec<_> = v.iter().filter(|s| s.dark < 0.0).collect();
        assert_eq!(tree.len(), 4);
        assert!(tree.iter().all(|s| s.pos == [2.5, 2.5] && s.height == 2.0));
        let corners: Vec<(f32, f32)> = tree.iter().map(|s| (s.along, s.edge[0])).collect();
        assert_eq!(corners, [(0.0, -1.0), (1.0, -1.0), (1.0, 1.0), (0.0, 1.0)]);
        let first = v.iter().position(|s| s.dark < 0.0).unwrap() as u32;
        assert_eq!(i.iter().filter(|&&k| k >= first).count(), 6, "two triangles");
    }

    #[test]
    fn deep_inside_a_mass_nothing_is_drawn() {
        let big = ["#########"; 9];
        let (v, _) = shapes(&big);
        // Only blocks within reach of the mass's edge have shapes.
        assert!(v.iter().all(|s| {
            let (x, y) = (s.pos[0], s.pos[1]);
            !(3.5..5.5).contains(&x) || !(3.5..5.5).contains(&y)
        }));
        let (empty, _) = shapes(&["...", "...", "..."]);
        assert!(empty.is_empty());
    }

    #[test]
    fn a_ridge_taller_than_its_mass_still_casts() {
        // A roof's ridge stands above its eaves: deep inside the house, but
        // its shadow reaches past them.
        let at = |x: i32, y: i32| match (x, y) {
            (4, 4) => Caster::Mass(3.0),
            (0..=8, 0..=8) => Caster::Mass(1.0),
            _ => Caster::None,
        };
        let (v, _) = build(0, 0, 10, 10, &at);
        assert!(v.iter().any(|s| s.height == 3.0), "the ridge is drawn");
        // Level mass inside, a neighbour as tall all round, is not.
        assert!(!v.iter().any(|s| s.height == 1.0 && (3.0..6.0).contains(&s.pos[0]) && (2.0..3.0).contains(&s.pos[1])));
    }

    #[test]
    fn rock_under_rock_takes_no_shadow_so_its_rim_casts_none() {
        // A mountain: its outer rim casts, and nothing beside the rock
        // under rock inside it does.
        let at = |x: i32, y: i32| match (x, y) {
            (4..=11, 4..=11) => Caster::Under(1.0),
            (0..=15, 0..=15) => Caster::Mass(1.0),
            _ => Caster::None,
        };
        let (v, _) = build(0, 0, 17, 17, &at);
        assert!(v.iter().any(|s| s.pos[0] < 1.0), "the outer rim casts");
        assert!(
            !v.iter().any(|s| (3.0..13.0).contains(&s.pos[0]) && (3.0..13.0).contains(&s.pos[1])),
            "the inner rim doesn't"
        );
    }

    #[test]
    fn neighbouring_chunks_draw_each_block_once() {
        let at =
            |x: i32, y: i32| if (2..6).contains(&x) && (2..6).contains(&y) { Caster::Mass(1.0) } else { Caster::None };
        let whole = build(0, 0, 8, 8, &at).0.len();
        let halves = build(0, 0, 4, 8, &at).0.len() + build(4, 0, 8, 8, &at).0.len();
        assert_eq!(whole, halves);
    }

    #[test]
    fn the_densest_block_fits_one_buffer() {
        // The buffers pack blocks into u16-indexed buffers, a block whole.
        let forest = |_: i32, _: i32| Caster::Crown(2.0);
        let checks = |x: i32, y: i32| if (x + y) % 2 == 0 { Caster::Mass(1.0) } else { Caster::None };
        for at in [&forest as &dyn Fn(i32, i32) -> Caster, &checks] {
            let (v, _) = build(0, 0, BLOCK, BLOCK, &|x, y| at(x, y));
            assert!(!v.is_empty() && v.len() <= MAX_VERTS, "{} vertices", v.len());
        }
    }
}
