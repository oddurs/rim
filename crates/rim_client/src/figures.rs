//! Plan figures on the GPU (DESIGN.md §6h, spike cc289956): every part of
//! every pawn (a disc or a rounded box, turned, filled and outlined in ink)
//! is a quad in one stream buffer, drawn in one call by a shader that works
//! out each shape's edge per pixel, crisp at any zoom and any turn.

use macroquad::miniquad::*;
use macroquad::prelude::{get_internal_gl, screen_height, screen_width, Color};
use rim_ui::body::{Body, Paint, Shape};

/// One part of a figure, in screen points.
#[derive(Clone, Copy, Debug)]
pub struct Part {
    pub centre: (f32, f32),
    /// Half width and half height before turning.
    pub half: (f32, f32),
    /// Radians, clockwise on screen.
    pub turn: f32,
    pub fill: Color,
    pub ink: Color,
    /// Outline width in points; 0 draws none.
    pub line: f32,
    /// 0 an ellipse; above 0 a box whose corners round by this share of
    /// its shorter half (1 is a capsule).
    pub round: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vert {
    pos: [f32; 2],
    /// Where this corner is in the part's own frame, in points.
    local: [f32; 2],
    half: [f32; 2],
    fill: [u8; 4],
    ink: [u8; 4],
    /// Outline width, and roundness (0 for an ellipse).
    shape: [f32; 2],
}

const VERTEX: &str = "#version 100
attribute vec2 pos;
attribute vec2 local0;
attribute vec2 half0;
attribute vec4 fill0;
attribute vec4 ink0;
attribute vec2 shape0;
varying vec2 local;
varying vec2 extent;
varying lowp vec4 fill;
varying lowp vec4 ink;
varying vec2 shape;
uniform vec2 screen;
uniform float flip;
void main() {
    vec2 p = pos / screen * 2.0 - 1.0;
    gl_Position = vec4(p.x, p.y * flip, 0.0, 1.0);
    local = local0;
    extent = half0;
    fill = fill0 / 255.0;
    ink = ink0 / 255.0;
    shape = shape0;
}";

const FRAGMENT: &str = "#version 100
precision mediump float;
varying vec2 local;
varying vec2 extent;
varying lowp vec4 fill;
varying lowp vec4 ink;
varying vec2 shape;
// Pixels a point: the world's target may be a fraction of the screen's
// pixels, and the edges are antialiased over a pixel, not a point.
uniform float px;
void main() {
    float d;
    if (shape.y <= 0.0) {
        // An ellipse: its implicit function over the length of its gradient,
        // a distance in points near the edge however long the ellipse is.
        float k = length(local / extent);
        float g = length(local / (extent * extent));
        d = g > 0.0 ? (k - 1.0) * k / g : -min(extent.x, extent.y);
    } else {
        float r = shape.y * min(extent.x, extent.y);
        vec2 q = abs(local) - extent + r;
        d = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
    }
    float cover = clamp(0.5 - d * px, 0.0, 1.0);
    float line = shape.x;
    // Inside the outline's band the ink lies over the fill, antialiased on
    // its inner edge. Premultiplied, so a clear fill leaves the ink its own
    // colour and a translucent ink shows the fill, not what's under it.
    float band = line > 0.0 ? clamp((d + line) * px + 0.5, 0.0, 1.0) : 0.0;
    float over = ink.a * band;
    vec4 c = vec4(ink.rgb * over + fill.rgb * fill.a * (1.0 - over), over + fill.a * (1.0 - over));
    gl_FragColor = c * cover;
}";

#[repr(C)]
struct Uniforms {
    screen: [f32; 2],
    flip: f32,
    px: f32,
}

/// Quads per buffer: u16 indices.
const MAX_PARTS: usize = (u16::MAX as usize + 1) / 4;

#[derive(Default)]
pub struct Batch {
    pipeline: Option<Pipeline>,
    buffers: Option<(BufferId, BufferId)>,
    verts: Vec<Vert>,
    indices: Vec<u16>,
    pub calls: usize,
    /// Time handing the batch to GL, macroquad's queue first: submission,
    /// counted with the rest of it (`RenderTimes::gl`), not as the pass.
    pub gl_us: f64,
}

impl Batch {
    pub fn clear(&mut self) {
        self.verts.clear();
        self.indices.clear();
    }

    pub fn len(&self) -> usize {
        self.verts.len() / 4
    }

    /// Indices drawn, for the bench's count.
    pub fn indices(&self) -> usize {
        self.indices.len()
    }

    /// Add a part. Past one buffer's worth (16,384 parts, some 1,200
    /// people at full detail) the rest are dropped for the frame.
    /// Can `n` more parts go in this frame?
    pub fn room(&self, n: usize) -> bool {
        self.len() + n <= MAX_PARTS
    }

    pub fn push(&mut self, p: &Part) {
        if !self.room(1) {
            return;
        }
        // Room for the antialiased edge: half a pixel, at down to a quarter
        // of a pixel a point.
        let (hx, hy) = (p.half.0 + 2.0, p.half.1 + 2.0);
        let (s, c) = p.turn.sin_cos();
        let n = self.verts.len() as u16;
        for (lx, ly) in [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)] {
            let pos = [p.centre.0 + lx * c - ly * s, p.centre.1 + lx * s + ly * c];
            self.verts.push(Vert {
                pos,
                local: [lx, ly],
                half: [p.half.0, p.half.1],
                fill: p.fill.into(),
                ink: p.ink.into(),
                shape: [p.line, p.round],
            });
        }
        self.indices.extend([n, n + 1, n + 2, n, n + 2, n + 3]);
    }

    fn pipeline(ctx: &mut dyn RenderingBackend) -> Pipeline {
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
                ShaderMeta {
                    images: vec![],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("screen", UniformType::Float2),
                            UniformDesc::new("flip", UniformType::Float1),
                            UniformDesc::new("px", UniformType::Float1),
                        ],
                    },
                },
            )
            .expect("the figure shader compiles");
        ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("pos", VertexFormat::Float2),
                VertexAttribute::new("local0", VertexFormat::Float2),
                VertexAttribute::new("half0", VertexFormat::Float2),
                VertexAttribute::new("fill0", VertexFormat::Byte4),
                VertexAttribute::new("ink0", VertexFormat::Byte4),
                VertexAttribute::new("shape0", VertexFormat::Float2),
            ],
            shader,
            PipelineParams {
                // The shader writes premultiplied colour.
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::One,
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
        )
    }

    /// Draw everything pushed since `clear`, in one call, after whatever
    /// macroquad has batched so far. `px` is pixels a point in what it
    /// draws into.
    pub fn draw(&mut self, target: Option<RenderPass>, px: f32) {
        if self.verts.is_empty() {
            return;
        }
        let start = std::time::Instant::now();
        // SAFETY: macroquad's GL context, used between its own flushes, as
        // mesh.rs does.
        let mut gl = unsafe { get_internal_gl() };
        gl.flush();
        let ctx = gl.quad_context;
        if self.pipeline.is_none() {
            self.pipeline = Some(Self::pipeline(ctx));
        }
        let cap = MAX_PARTS * 4;
        let (vb, ib) = *self.buffers.get_or_insert_with(|| {
            (
                ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<Vert>(cap)),
                ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Stream, BufferSource::empty::<u16>(cap / 4 * 6)),
            )
        });
        ctx.buffer_update(vb, BufferSource::slice(&self.verts));
        ctx.buffer_update(ib, BufferSource::slice(&self.indices));
        match target {
            Some(pass) => ctx.begin_pass(Some(pass), PassAction::Nothing),
            None => ctx.begin_default_pass(PassAction::Nothing),
        }
        ctx.apply_pipeline(self.pipeline.as_ref().expect("made above"));
        ctx.apply_bindings(&Bindings { vertex_buffers: vec![vb], index_buffer: ib, images: vec![] });
        let flip = if target.is_some() { 1.0 } else { -1.0 };
        ctx.apply_uniforms(UniformsSource::table(&Uniforms { screen: [screen_width(), screen_height()], flip, px }));
        ctx.draw(0, self.indices.len() as i32, 1);
        ctx.end_render_pass();
        self.calls += 1;
        self.gl_us += start.elapsed().as_secs_f64() * 1e6;
    }
}

/// How much of a figure is drawn at `z` points a cell (DESIGN.md §6h).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lod {
    /// A disc in the main colour.
    Dot,
    /// The parts that aren't `detail`: shadow, torso, head, hair.
    Silhouette,
    Full,
}

/// Below 10 points a cell a figure is a dot, below the worksite detail
/// zoom a silhouette, and from there whole.
pub fn lod(z: f32) -> Lod {
    if z < 10.0 {
        Lod::Dot
    } else if z < crate::worksite::DETAIL_ZOOM {
        Lod::Silhouette
    } else {
        Lod::Full
    }
}

/// The colours a creature fills its body's channels with.
#[derive(Clone, Copy, Debug)]
pub struct Paints {
    pub skin: Color,
    pub feet: Color,
    pub torso: Color,
    pub hair: Color,
}

impl Paints {
    fn of(&self, p: Paint) -> Color {
        match p {
            Paint::Rgba([r, g, b, a]) => Color::new(r, g, b, a),
            Paint::Skin => self.skin,
            Paint::Feet => self.feet,
            Paint::Torso => self.torso,
            Paint::Hair => self.hair,
        }
    }
}

/// A faction's ring, under the figure (DESIGN.md §6h).
fn faction_ring(b: &mut Batch, at: (f32, f32), r: f32, line: f32, c: Color) {
    b.push(&Part {
        centre: at,
        half: (r, r),
        turn: 0.0,
        fill: Color::new(0.0, 0.0, 0.0, 0.0),
        ink: c,
        line,
        round: 0.0,
    });
}

/// Far out, a creature is a dot in its main colour, with its faction's ring
/// drawn around it at a point and a half, so colonists and raiders still
/// differ at a glance.
fn dot(b: &mut Batch, at: (f32, f32), z: f32, c: Color, ring: Option<(Color, f32)>) {
    let r = (0.27 * z).max(2.0);
    b.push(&Part {
        centre: at,
        half: (r, r),
        turn: 0.0,
        fill: c,
        ink: ink(),
        line: rim_sim::look::Weight::Hair.px(z),
        round: 0.0,
    });
    if let Some((rc, rr)) = ring {
        let line = 1.5;
        faction_ring(b, at, rr.max(r + line), line, rc);
    }
}

/// A creature no body draws: a disc in its colour, `r` points across, with
/// its ring and levels of detail as a figure has them.
pub fn disc(b: &mut Batch, at: (f32, f32), r: f32, z: f32, c: Color, ring: Option<(Color, f32)>) {
    if lod(z) == Lod::Dot {
        dot(b, at, z, c, ring);
        return;
    }
    if let Some((rc, rr)) = ring {
        faction_ring(b, at, rr, rim_sim::look::Weight::Hair.px(z).max(1.0), rc);
    }
    let light = rim_sim::look::Weight::Light.px(z);
    b.push(&Part { centre: at, half: (r, r), turn: 0.0, fill: c, ink: ink(), line: light, round: 0.0 });
}

/// What doesn't match between bodies and creatures: a creature no body
/// draws (it is drawn as a disc, as every creature once was), and a body
/// that names a creature no mod defines, most often a typo.
pub fn unbodied(bodies: &rim_ui::body::Bodies, defs: &rim_sim::defs::DefDb) -> Vec<String> {
    let missing = defs
        .creatures
        .iter()
        .filter(|c| bodies.for_creature(&c.id).is_none())
        .map(|c| format!("creature {} has no [[body]] in any mod's ui/bodies.toml; it is drawn as a disc", c.id));
    let unknown = bodies.list.iter().flat_map(|b| {
        b.creatures
            .iter()
            .filter(|c| !defs.creatures.iter().any(|d| &d.id == *c))
            .map(move |c| format!("body {} draws {c}, which no loaded mod defines", b.id))
    });
    missing.chain(unknown).collect()
}

/// The plan's ink, as looks draw it.
fn ink() -> Color {
    let [r, g, b, a] = rim_sim::look::INK;
    Color::new(r, g, b, a)
}

/// One creature's figure, centred on screen point `at`, `z` points a cell,
/// facing `face` radians clockwise from north, with a faction `ring`
/// under it.
pub fn figure(
    b: &mut Batch,
    body: &Body,
    at: (f32, f32),
    z: f32,
    face: f32,
    paints: &Paints,
    ring: Option<(Color, f32)>,
) {
    let lod = lod(z);
    if lod == Lod::Dot {
        dot(b, at, z, paints.torso, ring);
        return;
    }
    if let Some((c, r)) = ring {
        faction_ring(b, at, r, rim_sim::look::Weight::Hair.px(z).max(1.0), c);
    }
    let parts = || body.parts.iter().filter(|p| lod == Lod::Full || !p.detail);
    // A figure is drawn whole or not at all: never a head without a body.
    if !b.room(parts().count()) {
        return;
    }
    let (s, c) = face.sin_cos();
    for p in parts() {
        let (x, y) = p.at;
        // A world part (the shadow) falls the same way whatever the creature
        // faces, but its shape still turns with it: a deer's long shadow
        // lies along the deer.
        let centre = if p.world {
            (at.0 + x * z, at.1 + y * z)
        } else {
            (at.0 + (x * c - y * s) * z, at.1 + (x * s + y * c) * z)
        };
        let turn = face;
        let (half, round) = match p.shape {
            Shape::Disc { r, squash } => ((r * z, r * squash * z), 0.0),
            Shape::Box { w, h, round } => ((w * z / 2.0, h * z / 2.0), round.max(1e-3)),
        };
        b.push(&Part {
            centre,
            half,
            turn,
            fill: paints.of(p.paint),
            ink: ink(),
            line: p.line.map_or(0.0, |w| w.px(z)),
            round,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_level_of_detail_switches_at_10_and_20_points() {
        assert_eq!(lod(4.0), Lod::Dot);
        assert_eq!(lod(9.99), Lod::Dot);
        assert_eq!(lod(10.0), Lod::Silhouette);
        assert_eq!(lod(19.99), Lod::Silhouette);
        assert_eq!(lod(20.0), Lod::Full);
        assert_eq!(lod(80.0), Lod::Full);
    }

    fn human() -> Body {
        let (bodies, errors) = rim_ui::body::parse("core", include_str!("../../../mods/core/ui/bodies.toml"));
        assert!(errors.is_empty(), "{errors:?}");
        bodies.into_iter().find(|b| b.id == "core:human").expect("core draws humans")
    }

    #[test]
    fn a_figure_is_a_quad_a_part_and_fewer_further_out() {
        let paints = Paints {
            skin: Color::new(1.0, 0.8, 0.6, 1.0),
            feet: Color::new(0.5, 0.4, 0.3, 1.0),
            torso: Color::new(0.5, 0.3, 0.2, 1.0),
            hair: Color::new(0.2, 0.1, 0.1, 1.0),
        };
        let body = human();
        let count = |z: f32| {
            let mut b = Batch::default();
            figure(&mut b, &body, (100.0, 100.0), z, 0.3, &paints, Some((Color::new(0.3, 0.7, 1.0, 1.0), z * 0.36)));
            b.len()
        };
        assert_eq!(count(40.0), body.parts.len() + 1, "every part and the ring, close up");
        assert_eq!(
            count(15.0),
            body.parts.iter().filter(|p| !p.detail).count() + 1,
            "no hands or feet as a silhouette"
        );
        assert_eq!(count(7.0), 2, "a dot and its ring, far out");
        // A batch with room for only part of a figure takes none of it.
        let mut full = Batch::default();
        let filler = Part {
            centre: (0.0, 0.0),
            half: (1.0, 1.0),
            turn: 0.0,
            fill: paints.skin,
            ink: paints.skin,
            line: 0.0,
            round: 0.0,
        };
        while full.room(3) {
            full.push(&filler);
        }
        let before = full.len();
        figure(&mut full, &body, (0.0, 0.0), 40.0, 0.0, &paints, None);
        assert_eq!(full.len(), before, "no half figures");
    }

    #[test]
    fn every_creature_of_the_shipped_mods_has_a_body_and_a_missing_one_warns() {
        let mods = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
        let loaded = rim_sim::modloader::load(&mods).expect("the mods load");
        let dirs: Vec<(String, &std::path::Path)> =
            loaded.mods.iter().map(|m| (m.id.clone(), m.dir.as_path())).collect();
        let bodies = rim_ui::body::Bodies::load(&dirs);
        assert!(bodies.warnings.is_empty(), "{:?}", bodies.warnings);
        assert_eq!(unbodied(&bodies, &loaded.defs), Vec::<String>::new());
        let none = rim_ui::body::Bodies::default();
        let w = unbodied(&none, &loaded.defs);
        assert!(w.iter().any(|w| w.contains("core:human") && w.contains("drawn as a disc")), "{w:?}");
        let dir = std::env::temp_dir().join(format!("rim-typo-body-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("ui")).unwrap();
        std::fs::write(
            dir.join("ui/bodies.toml"),
            "[[body]]\nid = \"h\"\ncreatures = [\"core:huamn\"]\nparts = [{ id = \"a\" }]",
        )
        .unwrap();
        let typo = rim_ui::body::Bodies::load(&[("m".into(), dir.as_path())]);
        let w = unbodied(&typo, &loaded.defs);
        assert!(w.iter().any(|w| w == "body m:h draws core:huamn, which no loaded mod defines"), "{w:?}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_part_turns_with_the_figure_and_the_shadow_does_not() {
        let paints = Paints {
            skin: Color::new(1.0, 0.8, 0.6, 1.0),
            feet: Color::new(0.5, 0.4, 0.3, 1.0),
            torso: Color::new(0.5, 0.3, 0.2, 1.0),
            hair: Color::new(0.2, 0.1, 0.1, 1.0),
        };
        let body = human();
        let mut north = Batch::default();
        let mut east = Batch::default();
        figure(&mut north, &body, (0.0, 0.0), 40.0, 0.0, &paints, None);
        figure(&mut east, &body, (0.0, 0.0), 40.0, std::f32::consts::FRAC_PI_2, &paints, None);
        let centre = |b: &Batch, i: usize| {
            let v = &b.verts[i * 4..i * 4 + 4];
            ((v[0].pos[0] + v[2].pos[0]) / 2.0, (v[0].pos[1] + v[2].pos[1]) / 2.0)
        };
        let head = body.parts.iter().position(|p| p.id == "head").unwrap();
        let (n, e) = (centre(&north, head), centre(&east, head));
        assert!(n.1 < -1.0 && n.0.abs() < 1e-3, "facing north the head is ahead, up the screen: {n:?}");
        assert!(e.0 > 1.0 && e.1.abs() < 1e-3, "facing east it is to the right: {e:?}");
        let shadow = body.parts.iter().position(|p| p.id == "shadow").unwrap();
        assert_eq!(centre(&north, shadow), centre(&east, shadow), "the shadow falls the same way whatever the facing");
        let corner = |b: &Batch, i: usize| b.verts[i * 4].pos;
        assert_ne!(corner(&north, shadow), corner(&east, shadow), "but its shape turns with the figure");
    }
}
