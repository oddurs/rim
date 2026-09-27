//! Light on screen (DESIGN.md §6e).
//!
//! The sim keeps one number per cell, the `light` field. The picture adds
//! where the sun reaches and how firelight colours a room, each computed at
//! the rate it changes, before the world is drawn:
//!
//! - **Occluders** (`occluders.rs`): what stops light, per cell, repacked as
//!   the map changes.
//! - **Sun**: each light texel steps toward the sun through the occluders'
//!   heights and stops at the first thing taller than the ray. Worked out
//!   again only when the sun moves a quarter of a degree or the occluders
//!   change; a still sky costs nothing.
//! - **Firelight**: the field's emitter stamps, rebuilt when they change.
//!
//! Then one multiply over the world adds them up: the sky's ambient light,
//! the sun where it reaches, a room's share of daylight indoors, firelight,
//! and the plan's contact shadow under every mass (0779def9), which fades as
//! the sun takes over.

use crate::occluders::Occluders;
use crate::sky::Air;
use crate::Cam;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;
use rim_sim::defs::SunPath;
use rim_sim::world::World;

/// Light texels per cell.
const TEXELS: u32 = 2;
/// Steps the sun march takes, 0.4 cells each: shadows reach 11 cells.
const SUN_STEPS: f32 = 28.0;
/// How far the sun moves before its shadows are worked out again, degrees.
const SUN_QUANT: f64 = 0.25;
/// The share of the sky's light that comes straight from the sun on a clear
/// day; the rest is the sky itself, which also reaches into shadow.
const DIRECT: f32 = 0.6;

const VERTEX: &str = "#version 100
precision lowp float;
attribute vec3 position;
attribute vec2 texcoord;
varying vec2 uv;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    uv = texcoord;
}";

// Where the sun reaches, one texel at a time: step toward it through the
// occluders' heights, rising tan(elevation) a cell, and stop at the first
// thing taller than the ray. The penumbra widens with distance from what
// casts it; a canopy lets dapples through. Sampled at cell centres, so the
// occluders' linear filter returns each cell exactly.
const SUN_FRAGMENT: &str = "#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying vec2 uv;
uniform sampler2D occluders;
uniform vec2 map;
uniform vec2 res;
uniform vec4 sun;
uniform float steps;
const float STEP = 0.4;
const float MAX_HEIGHT = 4.0;
float hash(vec2 p) {
    p = fract(p * vec2(443.897, 441.423));
    p += dot(p, p.yx + 19.19);
    return fract((p.x + p.y) * p.x);
}
vec4 cell(vec2 c) {
    return texture2D(occluders, (floor(c) + 0.5) / map);
}
// R's high six bits (occluders.rs); the low two are a window or a door.
float height(vec4 o) {
    return floor(floor(o.r * 255.0 + 0.5) / 4.0) / 63.0 * MAX_HEIGHT;
}
void main() {
    vec2 p = gl_FragCoord.xy / res * map;
    vec4 here = cell(p);
    if (here.g > 0.5 || sun.z <= 0.0) {
        gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
        return;
    }
    float h0 = here.b > 0.0 ? height(here) : 0.0;
    float vis = 1.0;
    float reach = steps * STEP;
    for (int i = 1; i <= 64; i++) {
        if (float(i) > steps) break;
        float t = float(i) * STEP;
        vec2 q = p + sun.xy * t;
        if (q.x < 0.0 || q.y < 0.0 || q.x >= map.x || q.y >= map.y) break;
        float h = h0 + t * sun.z;
        if (h > MAX_HEIGHT) break;
        vec4 o = cell(q);
        // Nothing lower than where the ray set out can shade it: a wall top
        // isn't shaded by the wall beside it, at any sun.
        if (o.b > 0.0 && height(o) > h0) {
            float stop = o.b > 0.99 ? 1.0 : clamp(o.b * (0.4 + 1.2 * hash(floor(q * 3.0))), 0.0, 1.0);
            float f = clamp(0.5 + (h - height(o)) / (sun.w * t + 0.015), 0.0, 1.0);
            float fade = 1.0 - smoothstep(0.7 * reach, reach, t);
            vis = min(vis, 1.0 - stop * (1.0 - f) * fade);
            if (vis < 0.004) break;
        }
    }
    gl_FragColor = vec4(vis, 0.0, 0.0, 1.0);
}";

// Multiplied over the world: the sky's ambient light, the sun where it
// reaches, a room's share of daylight indoors, and firelight where it is
// brighter, never darker than night. Indoors is the occluders' roof bit (G),
// which the linear filter softens where a floor meets its wall. Then the
// plan's contact shadow, down and to the right of every mass, fading as the
// sun reaches the ground there.
const MULTIPLY_FRAGMENT: &str = "#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying vec2 uv;
uniform sampler2D Texture;
uniform sampler2D occluders;
uniform sampler2D sunlit;
uniform vec3 ambient;
uniform vec3 direct;
uniform vec3 night;
uniform vec3 fire;
uniform float indoor_share;
uniform vec2 cell;
uniform float day;
// Whether the cell that point `p` falls in is a mass (a wall, rock, a
// window or door), decided at the cell's centre so it is all or nothing.
float mass_at(vec2 p) {
    vec4 o = texture2D(occluders, (floor(p / cell) + 0.5) * cell);
    return step(0.99, o.b) * (1.0 - step(0.5, o.g));
}
// What casts the plan's contact shadow from that cell: a mass fully, a
// canopy lightly, a roofed floor not at all.
float casts(vec2 p) {
    vec4 o = texture2D(occluders, (floor(p / cell) + 0.5) * cell);
    return (step(0.99, o.b) + 0.5 * step(0.5, o.b) * (1.0 - step(0.99, o.b))) * (1.0 - step(0.5, o.g));
}
void main() {
    vec4 o = texture2D(occluders, uv);
    float indoors = o.g;
    // Whether this cell is a mass is decided at its centre: a wall's own
    // edge, sampled between cells, is still wall. A mass's top takes the
    // sun of its own cell, so the shadow it casts doesn't creep up its side.
    vec2 centre = (floor(uv / cell) + 0.5) * cell;
    float solid = mass_at(uv);
    float sun = mix(texture2D(sunlit, uv).r, texture2D(sunlit, centre).r, solid);
    vec3 outside = ambient + direct * sun;
    vec3 inside = (ambient + direct) * indoor_share;
    vec3 c = max(max(night, mix(outside, inside, indoors)), fire * texture2D(Texture, uv).r);
    // A band a third of a cell wide below and right of every mass, crisp
    // like the plan's lines, its outer half lighter.
    float under = 0.5 * (casts(uv - cell * 0.18) + casts(uv - cell * 0.36)) * (1.0 - solid);
    c *= 1.0 - 0.3 * under * (1.0 - max(sun, day));
    gl_FragColor = vec4(min(c, vec3(1.0)), 1.0);
}";

/// One lighting pass's cost in the last frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PassTime {
    pub name: &'static str,
    /// Building the pass's draws, µs.
    pub cpu_us: f64,
    /// The GPU's time running them, µs; only with `gpu_timing`.
    pub gpu_us: Option<f64>,
    /// Whether it did its work this frame, or reused what it had.
    pub ran: bool,
    /// Draw calls it made.
    pub draws: u32,
}

/// The GPU timer, made the first time the bench asks for one.
#[derive(Default)]
enum Timer {
    #[default]
    Untried,
    /// GL has no timer, or none that can time a pass.
    Unavailable,
    Ready(GpuTimer),
}

/// `glGetString(GL_RENDERER)`: miniquad doesn't name the constant.
const GL_RENDERER: u32 = 0x1F01;

/// A `glGetString`, once a frame has been drawn; empty where GL has none.
fn gl_string(name: u32) -> String {
    // SAFETY: on the render thread with the context current; the pointer
    // is to a static, NUL-terminated string or null.
    unsafe {
        let s = miniquad::gl::glGetString(name);
        if s.is_null() {
            String::new()
        } else {
            std::ffi::CStr::from_ptr(s as _).to_string_lossy().into_owned()
        }
    }
}

/// Which GL draws the frame, for the bench to say where its numbers came from.
pub fn gl_renderer() -> String {
    gl_string(GL_RENDERER)
}

/// A GL timer query (`GL_TIME_ELAPSED`). miniquad's `ElapsedQuery` is a
/// stub in 0.4, so this calls GL directly.
struct GpuTimer(u32);

impl GpuTimer {
    /// One, if the context has a timer that can time one pass.
    fn new() -> Option<GpuTimer> {
        if !times_a_pass(&gl_string(miniquad::gl::GL_VERSION), &gl_renderer()) {
            return None;
        }
        let mut id = 0;
        // SAFETY: desktop GL 3.3 has query objects; on the render thread.
        unsafe { miniquad::gl::glGenQueries(1, &mut id) };
        (id != 0).then_some(GpuTimer(id))
    }

    fn begin(&mut self) {
        // SAFETY: a query made by `new`, and none other running: passes don't nest.
        unsafe { miniquad::gl::glBeginQuery(miniquad::gl::GL_TIME_ELAPSED, self.0) }
    }

    /// End the query and wait for its result, in µs.
    fn end(&mut self) -> f64 {
        use miniquad::gl::*;
        let mut ns: GLuint64 = 0;
        // SAFETY: ends the query `begin` started; reading GL_QUERY_RESULT
        // blocks until the GPU has run everything inside it.
        unsafe {
            glEndQuery(GL_TIME_ELAPSED);
            glGetQueryObjectui64v(self.0, GL_QUERY_RESULT, &mut ns);
        }
        ns as f64 / 1e3
    }
}

impl Drop for GpuTimer {
    fn drop(&mut self) {
        // SAFETY: a query `new` made, deleted once.
        unsafe { miniquad::gl::glDeleteQueries(1, &self.0) };
    }
}

/// Whether GL can time one pass: desktop GL 3.3 or later has timer queries
/// (GL ES and older desktop GL only as extensions), and the GPU draws in
/// order. A tile-based GPU (Apple's) runs a whole frame's tiles at once, so
/// a query around part of it times the tile pass, not the part.
fn times_a_pass(version: &str, renderer: &str) -> bool {
    if version.starts_with("OpenGL ES") || renderer.contains("Apple") {
        return false;
    }
    let mut it = version.split(|c: char| !c.is_ascii_digit()).filter_map(|n| n.parse::<u32>().ok());
    matches!((it.next(), it.next()), (Some(major), Some(minor)) if (major, minor) >= (3, 3))
}

/// Whether the GL draws in software: its "GPU" times are the CPU's.
pub fn software_gl(renderer: &str) -> bool {
    ["llvmpipe", "softpipe", "SwiftShader", "Software"].iter().any(|s| renderer.contains(s))
}

/// Hand macroquad's batched draws to GL, so a timer query brackets only
/// the draws made between two flushes.
fn flush_batches() {
    // SAFETY: called on the render thread between draws, where macroquad
    // flushes its own batches.
    unsafe { get_internal_gl() }.flush();
}

/// Where a sky body is at `hour`: azimuth and elevation in degrees. Below
/// the horizon the elevation is 0 or less.
pub fn sun_at(path: &SunPath, hour: f64) -> (f64, f64) {
    let span = (path.set - path.rise).rem_euclid(24.0);
    let f = (hour - path.rise).rem_euclid(24.0) / span;
    if f >= 1.0 {
        return (path.arc[1], -1.0);
    }
    (path.arc[0] + (path.arc[1] - path.arc[0]) * f, path.peak * (std::f64::consts::PI * f).sin())
}

/// How much of the sky's light comes straight from the sun: none below the
/// horizon, most of it on a clear day, and little under cloud.
fn direct_share(elevation: f64, cloud: f32) -> f32 {
    let t = ((elevation as f32 + 1.0) / 9.0).clamp(0.0, 1.0);
    DIRECT * t * t * (3.0 - 2.0 * t) * (1.0 - 0.8 * (cloud / 100.0).clamp(0.0, 1.0))
}

/// How fast the penumbra widens with distance from what casts it, per cell:
/// cloud scatters the sun into a wider, softer shadow.
fn penumbra(cloud: f32) -> f32 {
    0.03 + 0.15 * (cloud / 100.0).clamp(0.0, 1.0)
}

fn rgb3(c: [u8; 3]) -> Vec3 {
    vec3(c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0)
}

/// What the sun pass last worked out, so a still sky isn't worked out again.
#[derive(Clone, Copy, Debug, PartialEq)]
enum SunKey {
    /// Below the horizon: nothing is lit, whatever the cloud or the walls.
    Down,
    Up {
        /// Azimuth and elevation, in `SUN_QUANT` steps.
        az: i64,
        elev: i64,
        /// Penumbra growth, in fiftieths.
        soft: i64,
        occluders: u64,
    },
}

impl SunKey {
    fn new(sun: Option<(f64, f64)>, soft: f32, occluders: u64) -> SunKey {
        let q = |x: f64| (x / SUN_QUANT).round() as i64;
        match sun {
            Some((az, elev)) if q(elev) > 0 => {
                SunKey::Up { az: q(az), elev: q(elev), soft: (soft * 50.0).round() as i64, occluders }
            }
            _ => SunKey::Down,
        }
    }

    /// Toward the sun, in cells, and tan(elevation); zero when it's down.
    fn toward(self) -> Vec3 {
        match self {
            SunKey::Down => Vec3::ZERO,
            SunKey::Up { az, elev, .. } => {
                let (az, elev) = ((az as f64 * SUN_QUANT).to_radians(), (elev as f64 * SUN_QUANT).to_radians());
                vec3(az.cos() as f32, az.sin() as f32, elev.tan() as f32)
            }
        }
    }

    fn soft(self) -> f32 {
        match self {
            SunKey::Down => 0.0,
            SunKey::Up { soft, .. } => soft as f32 / 50.0,
        }
    }
}

#[derive(Default)]
pub struct Light {
    /// What stops light, per cell.
    pub occluders: Occluders,
    /// Firelight: the light field's emitter stamps, R.
    fire: Option<Texture2D>,
    /// The field stamp revision `fire` is current for.
    fire_key: Option<u64>,
    /// Where the sun reaches, R, `TEXELS` per cell.
    sunlit: Option<RenderTarget>,
    sun_key: Option<SunKey>,
    sun_material: Option<Material>,
    multiply_material: Option<Material>,
    /// Which shaders failed to build, sun and multiply: without the sun's,
    /// the world is lit without sun shadows; without the multiply, unlit.
    failed: [bool; 2],
    /// Where the sun reaches when the sun pass can't run: nowhere.
    no_sun: Option<Texture2D>,
    /// Pin the sun: (azimuth, elevation), degrees. For tests and the bench.
    pub pin_sun: Option<(f64, f64)>,
    /// Times the sun pass has run.
    pub sun_runs: u64,
    /// Each lighting pass last frame, in order.
    pub passes: Vec<PassTime>,
    /// Time each pass on the GPU with a timer query (`time_gpu`).
    gpu_timing: bool,
    query: Timer,
}

impl Light {
    /// Bring every cached result up to date. Call before the world is drawn,
    /// with the default camera: it draws into targets of its own.
    pub fn prepare(&mut self, w: &World, air: &Air) {
        self.passes.clear();
        let t = self.pass_begin();
        let changed = self.occluders.update(w);
        // Uploads, not draws.
        self.pass_end("occluders", t, changed, 0);
        let t = self.pass_begin();
        let rebuilt = self.update_fire(w);
        self.pass_end("firelight", t, rebuilt, 0);
        let t = self.pass_begin();
        let ran = self.update_sun(w, air);
        self.pass_end("sun", t, ran, ran as u32);
    }

    /// Multiply the world by the light. Call after everything lit is drawn.
    pub fn multiply(&mut self, w: &World, cam: &Cam, air: &Air, flash: f32) {
        let t = self.pass_begin();
        let drew = self.draw_multiply(w, cam, air, flash);
        self.pass_end("multiply", t, drew, drew as u32);
    }

    /// Forget every cached result, so the next frame rebuilds them all: what
    /// the render bench times as the cost of a change.
    pub fn invalidate(&mut self) {
        self.fire_key = None;
        self.sun_key = None;
        self.occluders.invalidate();
    }

    /// The sun's visibility at a point, in cells: the light texel it falls
    /// in, 0 to 1, read back from the GPU. Slow, for tests.
    pub fn sun_visibility(&self, x: f32, y: f32) -> Option<f32> {
        let rt = self.sunlit.as_ref()?;
        let img = rt.texture.get_texture_data();
        let (px, py) = ((x * TEXELS as f32).floor(), (y * TEXELS as f32).floor());
        (px >= 0.0 && py >= 0.0 && px < img.width as f32 && py < img.height as f32)
            .then(|| img.get_pixel(px as u32, py as u32).r)
    }

    /// Where the sun is now: pinned, or on the sky's path.
    fn sun(&self, w: &World) -> Option<(f64, f64)> {
        self.pin_sun.or_else(|| w.defs.sky.sun.as_ref().map(|p| sun_at(p, w.hour())))
    }

    fn material(&mut self, sun: bool) -> Option<Material> {
        if self.failed[sun as usize] {
            return None;
        }
        let slot = if sun { &mut self.sun_material } else { &mut self.multiply_material };
        if slot.is_none() {
            let m = if sun {
                load_material(
                    ShaderSource::Glsl { vertex: VERTEX, fragment: SUN_FRAGMENT },
                    MaterialParams {
                        uniforms: vec![
                            UniformDesc::new("map", UniformType::Float2),
                            UniformDesc::new("res", UniformType::Float2),
                            UniformDesc::new("sun", UniformType::Float4),
                            UniformDesc::new("steps", UniformType::Float1),
                        ],
                        textures: vec!["occluders".to_string()],
                        pipeline_params: PipelineParams { color_blend: None, ..Default::default() },
                    },
                )
            } else {
                load_material(
                    ShaderSource::Glsl { vertex: VERTEX, fragment: MULTIPLY_FRAGMENT },
                    MaterialParams {
                        uniforms: vec![
                            UniformDesc::new("ambient", UniformType::Float3),
                            UniformDesc::new("direct", UniformType::Float3),
                            UniformDesc::new("night", UniformType::Float3),
                            UniformDesc::new("fire", UniformType::Float3),
                            UniformDesc::new("indoor_share", UniformType::Float1),
                            UniformDesc::new("cell", UniformType::Float2),
                            UniformDesc::new("day", UniformType::Float1),
                        ],
                        textures: vec!["occluders".to_string(), "sunlit".to_string()],
                        pipeline_params: PipelineParams {
                            // Multiply: result = source × destination.
                            color_blend: Some(BlendState::new(
                                Equation::Add,
                                BlendFactor::Value(BlendValue::DestinationColor),
                                BlendFactor::Zero,
                            )),
                            ..Default::default()
                        },
                    },
                )
            };
            match m {
                Ok(m) => *slot = Some(m),
                Err(e) => {
                    let without = if sun { "without sun shadows" } else { "unlit" };
                    eprintln!("lighting shader failed, drawing {without}: {e}");
                    self.failed[sun as usize] = true;
                    return None;
                }
            }
        }
        slot.clone()
    }

    /// Rebuild the firelight texture if emitters changed. Whether it did.
    fn update_fire(&mut self, w: &World) -> bool {
        let key = w.fields.revision;
        if self.fire.is_some() && self.fire_key == Some(key) {
            return false;
        }
        self.fire_key = Some(key);
        let Some(light) = w.defs.lookup("field", "light") else { return false };
        let stamped = &w.fields.layers[light as usize].stamped;
        let (mw, mh) = (w.map.w as usize, w.map.h as usize);
        let mut bytes = vec![0u8; mw * mh * 4];
        for i in 0..mw * mh {
            let fire = (stamped[i] as f32 / rim_sim::field::FIXED as f32 / 100.0).clamp(0.0, 1.0);
            bytes[i * 4] = (fire * 255.0) as u8;
            bytes[i * 4 + 3] = 255;
        }
        let img = Image { bytes, width: mw as u16, height: mh as u16 };
        match &self.fire {
            Some(t) if t.width() as usize == mw && t.height() as usize == mh => t.update(&img),
            _ => {
                let t = Texture2D::from_image(&img);
                t.set_filter(FilterMode::Linear);
                self.fire = Some(t);
            }
        }
        true
    }

    /// Work out where the sun reaches, if it moved or the occluders changed.
    /// Whether it did.
    fn update_sun(&mut self, w: &World, air: &Air) -> bool {
        let Some(occ) = self.occluders.texture.clone() else { return false };
        let size = (w.map.w as u32 * TEXELS, w.map.h as u32 * TEXELS);
        let key = SunKey::new(self.sun(w), penumbra(air.cloud), self.occluders.version);
        let fits = self.sunlit.as_ref().is_some_and(|t| (t.texture.width() as u32, t.texture.height() as u32) == size);
        if fits && self.sun_key == Some(key) {
            return false;
        }
        let Some(m) = self.material(true) else { return false };
        if !fits {
            let rt = render_target(size.0, size.1);
            rt.texture.set_filter(FilterMode::Linear);
            self.sunlit = Some(rt);
        }
        self.sun_key = Some(key);
        // The key's quantised sun, so the result is exactly the key's.
        let (toward, soft) = (key.toward(), key.soft());
        let (tw, th) = (size.0 as f32, size.1 as f32);
        m.set_texture("occluders", occ);
        m.set_uniform("map", vec2(w.map.w as f32, w.map.h as f32));
        m.set_uniform("res", vec2(tw, th));
        m.set_uniform("sun", toward.extend(soft));
        m.set_uniform("steps", SUN_STEPS);
        set_camera(&Camera2D {
            zoom: vec2(2.0 / tw, 2.0 / th),
            target: vec2(tw / 2.0, th / 2.0),
            render_target: self.sunlit.clone(),
            ..Default::default()
        });
        gl_use_material(&m);
        draw_rectangle(0.0, 0.0, tw, th, WHITE);
        gl_use_default_material();
        set_default_camera();
        self.sun_runs += 1;
        true
    }

    /// The light on something outdoors, above every shadow: the sky, never
    /// darker than night. Roofs are drawn in it.
    pub fn outdoor(w: &World, air: &Air, flash: f32) -> Vec3 {
        Self::sky_color(w, air, flash).max(rgb3(w.defs.sky.rgb_night))
    }

    /// The sky's colour and brightness now: white, tinted by `[[sky]]`
    /// tints, times the outdoor light, brightened by a lightning flash.
    fn sky_color(w: &World, air: &Air, flash: f32) -> Vec3 {
        let sky = &w.defs.sky;
        let mut c = Vec3::ZERO;
        let mut total = 0.0;
        for t in sky.tint.values() {
            let s = (w.fields.eval_global(&t.strength) as f32).clamp(0.0, 1.0);
            c += rgb3(t.rgb) * s;
            total += s;
        }
        if total > 1.0 {
            c /= total;
            total = 1.0;
        }
        let tinted = c + Vec3::ONE * (1.0 - total);
        // Perceived brightness: an overcast day at half the light still
        // looks like day.
        let bright = (air.light / 100.0).clamp(0.0, 1.44).sqrt() + flash;
        tinted * bright
    }

    /// Draw the light over the world with multiply blending. Whether it
    /// drew: without the shader the world stays unlit.
    fn draw_multiply(&mut self, w: &World, cam: &Cam, air: &Air, flash: f32) -> bool {
        let (Some(fire), Some(occ)) = (self.fire.clone(), self.occluders.texture.clone()) else { return false };
        // Without the sun pass, the sun reaches nowhere.
        let sun = match &self.sunlit {
            Some(t) => t.texture.clone(),
            None => self.no_sun.get_or_insert_with(|| Texture2D::from_rgba8(1, 1, &[0, 0, 0, 255])).clone(),
        };
        let Some(m) = self.material(false) else { return false };
        let sky = Self::sky_color(w, air, flash);
        let elev = self.sun(w).map_or(-1.0, |s| s.1);
        let share = direct_share(elev, air.cloud);
        let def = &w.defs.sky;
        let (mw, mh) = (w.map.w as f32, w.map.h as f32);
        m.set_texture("occluders", occ);
        m.set_texture("sunlit", sun);
        m.set_uniform("ambient", sky * (1.0 - share));
        m.set_uniform("direct", sky * share);
        m.set_uniform("night", rgb3(def.rgb_night));
        m.set_uniform("fire", rgb3(def.rgb_fire) * 1.2);
        m.set_uniform("indoor_share", def.indoor_share as f32);
        m.set_uniform("cell", vec2(1.0 / mw, 1.0 / mh));
        // With no sun path the contact shadow fades with daylight instead.
        let day = if def.sun.is_none() { (air.light / 100.0).clamp(0.0, 1.0) } else { 0.0 };
        m.set_uniform("day", day);
        gl_use_material(&m);
        let (sx, sy) = cam.to_screen(0.0, 0.0);
        draw_texture_ex(
            &fire,
            sx,
            sy,
            WHITE,
            DrawTextureParams { dest_size: Some(vec2(mw * cam.zoom, mh * cam.zoom)), ..Default::default() },
        );
        gl_use_default_material();
        true
    }

    /// Time each pass on the GPU. Only the render bench does: reading a
    /// timer back waits for the GPU, which a real frame must never do.
    pub fn time_gpu(&mut self, on: bool) {
        self.gpu_timing = on;
    }

    /// The GPU timer, where GL has one and the bench asked for it.
    fn timer(&mut self) -> Option<&mut GpuTimer> {
        if !self.gpu_timing {
            return None;
        }
        if matches!(self.query, Timer::Untried) {
            self.query = GpuTimer::new().map_or(Timer::Unavailable, Timer::Ready);
        }
        match &mut self.query {
            Timer::Ready(t) => Some(t),
            _ => None,
        }
    }

    fn pass_begin(&mut self) -> std::time::Instant {
        if let Some(q) = self.timer() {
            flush_batches();
            q.begin();
        }
        std::time::Instant::now()
    }

    fn pass_end(&mut self, name: &'static str, start: std::time::Instant, ran: bool, draws: u32) {
        let cpu_us = start.elapsed().as_secs_f64() * 1e6;
        let gpu_us = self.timer().map(|q| {
            flush_batches();
            q.end()
        });
        self.passes.push(PassTime { name, cpu_us, gpu_us, ran, draws });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_never_waits_for_the_gpu_unless_the_bench_asks() {
        // Waiting stalls the pipeline; only the render bench may.
        let mut light = Light::default();
        assert!(!light.gpu_timing);
        let t = light.pass_begin();
        light.pass_end("pass", t, true, 1);
        assert_eq!(light.passes[0].gpu_us, None);
        assert!(matches!(light.query, Timer::Untried), "no timer is even made");
    }

    #[test]
    fn passes_are_timed_only_where_gl_can_time_one() {
        assert!(times_a_pass("3.3 (Core Profile) Mesa 24.0.9", "llvmpipe (LLVM 17.0.6, 256 bits)"));
        assert!(times_a_pass("4.6.0 NVIDIA 550.54", "NVIDIA GeForce GTX 1650/PCIe/SSE2"));
        assert!(times_a_pass("4.6 (Core Profile) Mesa 23.2.1", "Mesa Intel(R) UHD Graphics 620 (KBL GT2)"));
        assert!(!times_a_pass("4.1 Metal - 89.3", "Apple M2"), "tile-based: a query times the tile pass");
        assert!(!times_a_pass("3.2 Mesa 20.0", "llvmpipe"));
        assert!(!times_a_pass("OpenGL ES 3.2 Mesa", "Mali-G78"));
        assert!(!times_a_pass("", ""));
        assert!(software_gl("llvmpipe (LLVM 17.0.6, 256 bits)"));
        assert!(!software_gl("Mesa Intel(R) UHD Graphics 620 (KBL GT2)"));
    }

    #[test]
    fn the_sun_rises_in_the_east_peaks_at_noon_and_sets() {
        let path = SunPath { rise: 5.0, set: 21.0, peak: 60.0, arc: [-10.0, 190.0] };
        let (az, elev) = sun_at(&path, 5.0);
        assert!((az + 10.0).abs() < 1e-9 && elev.abs() < 1e-9, "on the horizon at rise");
        let (az, elev) = sun_at(&path, 13.0);
        assert!((az - 90.0).abs() < 1e-9 && (elev - 60.0).abs() < 1e-9, "due south, highest, halfway");
        assert!(sun_at(&path, 23.0).1 < 0.0 && sun_at(&path, 2.0).1 < 0.0, "down at night");
        let wrap = SunPath { rise: 20.0, set: 4.0, peak: 30.0, arc: [0.0, 180.0] };
        assert!((sun_at(&wrap, 0.0).1 - 30.0).abs() < 1e-9, "a path may cross midnight");
    }

    #[test]
    fn a_sun_below_the_horizon_is_worked_out_once_whatever_the_weather() {
        assert_eq!(SunKey::new(Some((180.0, -5.0)), 0.18, 7), SunKey::new(None, 0.03, 9), "down is down");
        assert_eq!(SunKey::new(Some((90.0, 0.1)), 0.03, 1), SunKey::Down, "under a quarter degree is down");
        let up = SunKey::new(Some((90.0, 30.0)), 0.03, 1);
        assert_eq!(up, SunKey::new(Some((90.1, 30.1)), 0.035, 1), "small moves and cloud drift reuse it");
        assert_ne!(up, SunKey::new(Some((90.3, 30.0)), 0.03, 1), "a quarter degree works it out again");
        assert_ne!(up, SunKey::new(Some((90.0, 30.0)), 0.03, 2), "so do changed walls");
        assert_eq!(SunKey::Down.toward(), Vec3::ZERO);
    }

    #[test]
    fn the_sun_gives_most_of_a_clear_day_and_little_under_cloud() {
        assert_eq!(direct_share(-5.0, 0.0), 0.0);
        assert!((direct_share(45.0, 0.0) - DIRECT).abs() < 1e-6);
        assert!(direct_share(45.0, 100.0) < DIRECT * 0.25);
        assert!(penumbra(100.0) > penumbra(0.0) * 4.0, "cloud softens shadows");
    }
}
