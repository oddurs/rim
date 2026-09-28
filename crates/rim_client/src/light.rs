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
//! - **Firelight**: every light-giving thing baked once with soft shadows,
//!   eight rays across its flame, into one of four flicker channels. Baked
//!   again only when a light or a wall changes; flicker is four colours a
//!   frame, however many lights there are.
//!
//! Then one multiply over the world adds them up: the sky's ambient light,
//! the sun where it reaches, a room's share of daylight indoors, firelight,
//! and the plan's contact shadow under every mass (0779def9), which fades as
//! the sun takes over.

use crate::occluders::Occluders;
use crate::quality::{texels_for, Setting};
use crate::sky::{Air, Flash};
use crate::Cam;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;
use rim_sim::defs::{Flicker, SunPath};
use rim_sim::map::Map;
use rim_sim::world::{Thing, World};

// What the presets tune (texels a cell, sun steps, softness, when the sun
// is worked out again, upsampling) is `crate::quality`.
/// A flash brighter than this casts shadows: the sun pass works them out
/// once as it strikes and once as it fades, whatever it lasts.
const FLASH_ON: f32 = 0.1;
/// Where a bolt lights from: high, so its shadows are short and hard.
const FLASH_ELEVATION: f64 = 40.0;
/// A bolt's light, blue-white, at full strength.
const FLASH_RGB: Vec3 = vec3(0.8, 0.88, 1.0);
/// What of a flash lights everything, shadow or not: the cloud it lights.
const FLASH_AMBIENT: f32 = 0.35;
/// The share of the sky's light that comes straight from the sun on a clear
/// day; the rest is the sky itself, which also reaches into shadow.
const DIRECT: f32 = 0.6;
/// The steady channel: lights that glow without flickering.
const STEADY: usize = 3;
/// Baked firelight is stored at this fraction, so overlapping fires can add
/// up past full before the 8-bit target clips; both shaders take it.
const FIRE_SCALE: f32 = 0.6;
/// The farthest a light's shadows are traced, in cells: the bake's march
/// takes at most 95 steps. Longer reaches are cut to it.
pub const MAX_REACH: f32 = 23.0;

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
// R's high six bits (occluders.rs); the low two are a window (1) or a door (2).
float height(vec4 o) {
    return floor(floor(o.r * 255.0 + 0.5) / 4.0) / 63.0 * MAX_HEIGHT;
}
float kind(vec4 o) {
    return mod(floor(o.r * 255.0 + 0.5), 4.0);
}
// A roof sits a storey up, on its walls; a window's pane runs from sill to
// lintel, and lets most of the sun through.
const float ROOF = 1.0;
const float SILL = 0.28;
const float LINTEL = 0.92;
const float GLASS = 0.85;
void main() {
    vec2 p = gl_FragCoord.xy / res * map;
    vec4 here = cell(p);
    if (sun.z <= 0.0) {
        gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
        return;
    }
    // Under a roof, the sun gets in only through a window, between its sill
    // and lintel. The roof rests on what bounds the room, so a ray that
    // leaves any other way (a wall, a door, or water that closes a room
    // without a wall) stops there, however high it has climbed.
    bool inside = here.g > 0.5;
    float h0 = (!inside && here.b > 0.0) ? height(here) : 0.0;
    float vis = 1.0;
    float reach = steps * STEP;
    vec2 pane = vec2(-1.0);
    for (int i = 1; i <= 64; i++) {
        if (float(i) > steps) break;
        float t = float(i) * STEP;
        vec2 q = p + sun.xy * t;
        if (q.x < 0.0 || q.y < 0.0 || q.x >= map.x || q.y >= map.y) break;
        float h = h0 + t * sun.z;
        vec4 o = cell(q);
        if (inside) {
            if (o.g > 0.5) {
                if (h >= ROOF) { vis = 0.0; break; }
                continue;
            }
            if (o.b > 0.99 && kind(o) == 1.0 && h > SILL && h < LINTEL) {
                vis *= GLASS;
                inside = false;
                pane = floor(q);
                continue;
            }
            vis = 0.0;
            break;
        }
        if (h > MAX_HEIGHT) break;
        // Out through the pane: the rest of its own cell doesn't shade it.
        if (floor(q) == pane) continue;
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
    // Still under the roof when the steps ran out: it never saw the sky.
    if (inside) vis = 0.0;
    gl_FragColor = vec4(vis, 0.0, 0.0, 1.0);
}";

// One light-giving thing per quad, all in one draw: its centre, reach and
// strength ride in the vertex's spare `normal`, and its flicker channel in
// its colour, which masks what it adds.
const BAKE_VERTEX: &str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;
varying vec4 lamp;
varying vec4 channel;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    lamp = normal;
    // Vertex colours arrive as bytes, 0 to 255.
    channel = color0 / 255.0;
}";

// A light's glow at one texel: close to inverse-square near the flame, zero
// at its reach, and shadowed along eight rays spread across the flame, each
// stopped as much as the occluders' firelight opacity (A) says. A texel's
// own half cell and the flame's own are skipped, so a wall's face lights up.
const BAKE_FRAGMENT: &str = "#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying vec4 lamp;
varying vec4 channel;
uniform sampler2D occluders;
uniform vec2 map;
uniform vec2 res;
uniform float scale;
const float STEP = 0.25;
const float FLAME = 0.3;
// A cell lets through what its opacity (A) doesn't stop, per cell crossed,
// so a window of pass 0.35 passes 0.35 whatever the angle. A texel's own
// half cell is skipped, so a wall's face lights up, and so is the flame's
// own cell, so a light set in a wall isn't shaded by it.
float trace(vec2 a, vec2 b) {
    vec2 dv = b - a;
    float len = length(dv);
    float n = min(ceil(len / STEP), 95.0);
    float step = len / n;
    float through = 1.0;
    for (int i = 1; i < 96; i++) {
        if (float(i) >= n) break;
        float s = float(i) / n;
        float from = s * len;
        if (from < 0.45 || len - from < 0.5 + FLAME) continue;
        float stop = texture2D(occluders, (a + dv * s) / map).a;
        through *= pow(max(1.0 - stop, 0.0), step);
        if (through < 0.01) return 0.0;
    }
    return through;
}
void main() {
    // A room's fill: flat, its value already in its colour.
    if (lamp.z <= 0.0) {
        gl_FragColor = channel;
        return;
    }
    vec2 p = gl_FragCoord.xy / res * map;
    vec2 dv = lamp.xy - p;
    float d = length(dv);
    if (d > lamp.z) discard;
    float x = d / lamp.z;
    float x2 = x * x;
    float fall = (1.0 - x2 * x2);
    fall = fall * fall / (1.0 + 0.08 * d * d);
    vec2 across = d > 0.001 ? vec2(-dv.y, dv.x) / d : vec2(0.0);
    float lit = 0.0;
    for (int r = 0; r < 8; r++) {
        lit += trace(p, lamp.xy + across * (float(r) / 7.0 - 0.5) * 2.0 * FLAME);
    }
    gl_FragColor = channel * (lamp.w * fall * lit / 8.0 * scale);
}";

// Zero a light target where a partial bake redraws, under a scissor.
const CLEAR_FRAGMENT: &str = "#version 100
precision lowp float;
void main() {
    gl_FragColor = vec4(0.0);
}";

// Multiplied over the world: the sky's ambient light, the sun where it
// reaches, a room's share of daylight indoors, and firelight where it is
// brighter (the baked channels, each its colour times its flicker now),
// never darker than night. Indoors is the occluders' roof bit (G),
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
uniform sampler2D rooms;
uniform float exposure;
uniform vec3 ambient;
uniform vec3 direct;
uniform vec3 bolt;
uniform vec3 night;
uniform vec3 ch0;
uniform vec3 ch1;
uniform vec3 ch2;
uniform vec3 ch3;
uniform float scale;
uniform vec2 cell;
uniform vec2 lres;
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
    // Indoors, a room's share of the sky through its walls and windows,
    // and the sun itself where it comes through a pane: from this texel
    // alone, so the linear filter doesn't carry the sun on the wall's
    // outer face onto the floor inside.
    float share = texture2D(rooms, uv).r;
    float sun_in = texture2D(sunlit, (floor(uv * lres) + 0.5) / lres).r;
    // A lightning flash, while it lasts, is what `sunlit` holds.
    vec3 outside = ambient + (direct + bolt) * sun;
    vec3 inside = (ambient + direct) * share + (direct + bolt) * sun_in;
    vec4 f = texture2D(Texture, uv) / scale;
    vec3 fire = f.r * ch0 + f.g * ch1 + f.b * ch2 + f.a * ch3;
    // Firelight adds to the sky, and the eye's exposure scales both: a fire
    // reads strong at night and weak at noon because the eye adapts.
    vec3 c = max(night, (mix(outside, inside, indoors) + fire) * exposure);
    float under = 0.5 * (casts(uv - cell * 0.18) + casts(uv - cell * 0.36)) * (1.0 - solid);
    c *= 1.0 - 0.3 * under * (1.0 - max(sun, day));
    gl_FragColor = vec4(c, 1.0);
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

/// How brightly a roof slope is lit, north, west, east, south and flat, as
/// a factor on the outdoor light. The sun's `share` of it falls on a slope
/// by how squarely the slope faces the sun, against flat ground's, so a
/// flat roof is lit as the ground is; a slope turned from a low sun gets the
/// sky alone. Without a sun, a fixed light from the north-west, as roofs had
/// before there was one, and in between the two by how much of the light
/// is the sun's: no jump at sunset, and relief under cloud.
pub fn roof_faces(sun: Option<(f64, f64)>, share: f32) -> [f32; 5] {
    const UNLIT: [f32; 5] = [1.14, 1.02, 0.86, 0.74, 1.0];
    /// The most a slope facing a low sun gets, against flat ground.
    const FACING: f32 = 1.6;
    let Some((az, elev)) = sun.filter(|s| s.1 > 0.0) else { return UNLIT };
    let k = (share / DIRECT).clamp(0.0, 1.0);
    let (az, elev) = (az.to_radians() as f32, elev.to_radians() as f32);
    let s = vec3(elev.cos() * az.cos(), elev.cos() * az.sin(), elev.sin());
    let p = crate::occluders::ROOF_PITCH as f32;
    // Each slope descends toward its side: its normal leans that way.
    let lit = |face: usize, dx: f32, dy: f32| {
        let n = vec3(p * dx, p * dy, 1.0).normalize();
        let sunlit = (1.0 - share) + share * (n.dot(s) / s.z).clamp(0.0, FACING);
        UNLIT[face] + (sunlit - UNLIT[face]) * k
    };
    [lit(0, 0.0, -1.0), lit(1, -1.0, 0.0), lit(2, 1.0, 0.0), lit(3, 0.0, 1.0), 1.0]
}

fn rgb3(c: [u8; 3]) -> Vec3 {
    vec3(c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0)
}

/// The lighting passes that draw with a material of their own.
#[derive(Clone, Copy)]
enum Pass {
    Sun = 0,
    Bake = 1,
    Multiply = 2,
    Clear = 3,
}

/// A light-giving thing, as the bake draws it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Lamp {
    /// Its centre, in cells.
    x: f32,
    y: f32,
    /// How far it reaches, in cells, and how bright it is at the source, 0 to 1.
    reach: f32,
    strength: f32,
    channel: usize,
}

impl Lamp {
    /// The cells its glow can touch: x0, y0, x1, y1.
    fn area(&self) -> [f32; 4] {
        let r = self.reach + 0.5;
        [self.x - r, self.y - r, self.x + r, self.y + r]
    }

    fn touches(&self, a: [f32; 4]) -> bool {
        let b = self.area();
        b[0] <= a[2] && b[2] >= a[0] && b[1] <= a[3] && b[3] >= a[1]
    }
}

/// What a firelight bake has to redo.
#[derive(Debug, PartialEq)]
enum Redo {
    Nothing,
    Whole,
    /// These areas, in cells: x0, y0, x1, y1.
    Areas(Vec<[f32; 4]>),
}

/// What changed since `baked`, as areas to redo: around each light that
/// came, went or changed, and around each light near a wall that moved.
/// `walls` is the occluders' changed rectangles, `Some(&[])` when they all
/// changed, `None` when none did.
fn redo(baked: &[Lamp], lamps: &[Lamp], walls: Option<&[(i32, i32, i32, i32)]>) -> Redo {
    let came = lamps.iter().filter(|l| !baked.contains(l));
    let went = baked.iter().filter(|l| !lamps.contains(l));
    let mut areas: Vec<[f32; 4]> = came.chain(went).map(Lamp::area).collect();
    match walls {
        Some([]) => return Redo::Whole,
        Some(rects) => {
            let cells = rects.iter().map(|&(x, y, w, h)| [x as f32, y as f32, (x + w) as f32, (y + h) as f32]);
            let cells: Vec<[f32; 4]> = cells.collect();
            areas.extend(lamps.iter().filter(|l| cells.iter().any(|&c| l.touches(c))).map(Lamp::area));
        }
        None => {}
    }
    if areas.is_empty() {
        Redo::Nothing
    } else {
        Redo::Areas(areas)
    }
}

/// The flicker channel for a light at `(x, y)`: steady lights share one; a
/// fire takes one of three phases by where it stands, so no two fires side
/// by side (or above and below) pulse in step.
fn channel_of(flicker: Flicker, x: i32, y: i32) -> usize {
    match flicker {
        Flicker::Steady => STEADY,
        Flicker::Fire => (x + y).rem_euclid(3) as usize,
    }
}

/// Every light-giving thing in the world, from the light field's emitters.
fn lamps(w: &World) -> Vec<Lamp> {
    let Some(field) = w.defs.lookup("field", "light") else { return Vec::new() };
    w.fields
        .emitters_of(field as usize)
        .filter(|&(_, _, amount, _)| amount > 0.0)
        .map(|(e, p, amount, radius)| {
            let def = w.ecs.get::<&Thing>(e).ok().map(|t| t.def);
            let flicker =
                def.and_then(|d| w.defs.things[d as usize].glow.as_ref()).map_or(Flicker::Fire, |g| g.flicker);
            Lamp {
                x: p.x as f32 + 0.5,
                y: p.y as f32 + 0.5,
                // A light that reaches no further than its own cell still lights it.
                reach: (radius as f32).clamp(0.75, MAX_REACH),
                strength: (amount as f32 / 100.0).clamp(0.0, 1.0 / FIRE_SCALE),
                channel: channel_of(flicker, p.x, p.y),
            }
        })
        .collect()
}

/// Lights per mesh: macroquad caps a draw at 16,000 vertices and 24,000
/// indices (`draw_call_*_capacity` in main.rs), four and six a light.
const LAMPS_PER_MESH: usize = 4_000;

/// The bake's quads, `texels` a cell, a light's centre, reach and strength in
/// each vertex's `normal` and its channel as the colour it adds to.
fn lamp_meshes(lamps: &[Lamp], texels: f32) -> Vec<Mesh> {
    lamps
        .chunks(LAMPS_PER_MESH)
        .map(|chunk| {
            let mut vertices = Vec::with_capacity(chunk.len() * 4);
            let mut indices = Vec::with_capacity(chunk.len() * 6);
            for l in chunk {
                let mut colour = [0u8; 4];
                colour[l.channel] = 255;
                let normal = vec4(l.x, l.y, l.reach, l.strength);
                let r = l.reach + 0.5;
                let base = vertices.len() as u16;
                for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                    let position = vec3((l.x + dx * r) * texels, (l.y + dy * r) * texels, 0.0);
                    vertices.push(Vertex { position, uv: Vec2::ZERO, color: colour, normal });
                }
                indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            }
            Mesh { vertices, indices, texture: None }
        })
        .collect()
}

/// A little smooth noise, 0 to 1, for flames: the same `t` gives the same value.
fn noise(t: f64) -> f32 {
    let hash = |i: i64| {
        let mut h = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 31;
        h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        (h >> 40) as f32 / (1u64 << 24) as f32
    };
    let (i, f) = (t.floor() as i64, t.fract() as f32);
    let u = f * f * (3.0 - 2.0 * f);
    hash(i) + (hash(i + 1) - hash(i)) * u
}

/// How bright a flame is at time `t`, around 1: two octaves of smooth noise
/// and a rare gust that dips it. Never a fresh random value a frame, which
/// reads as a strobe.
fn flicker(t: f64, seed: f64) -> f32 {
    let n = noise(t * 6.5 + seed) * 0.6 + noise(t * 14.0 + seed * 2.1) * 0.4;
    let gust = (noise(t * 0.8 + seed * 0.37) - 0.6).max(0.0) * 2.2;
    1.0 + 0.28 * n - 0.3 * gust - 0.14
}

/// A channel's colour now: firelight times its flame, redder as it dims.
/// The steady channel doesn't flicker.
fn channel_colour(fire: Vec3, channel: usize, t: f64) -> Vec3 {
    if channel == STEADY {
        return fire;
    }
    let f = flicker(t, channel as f64 * 17.3 + 3.1);
    vec3(fire.x * f, fire.y * f * (0.86 + 0.14 * f), fire.z * f * (0.72 + 0.28 * f))
}

/// How far the eye opens up for a sky this bright: not at all by day, up to
/// 2.6 times on a moonless night. Firelight is scaled with it, which is why a
/// fire reads strong at night and weak at noon.
fn exposure_for(sky: Vec3) -> f32 {
    let lum = 0.2126 * sky.x + 0.7152 * sky.y + 0.0722 * sky.z;
    (0.75 / lum.max(0.02)).powf(0.45).clamp(1.0, 2.6)
}

/// How much light a closed room sends back from its walls, as a share of
/// what its lights put out: flat over the room, in each light's channel.
const BOUNCE: f32 = 0.3;
/// The most a room's fill adds, so a fire in a cupboard isn't a floodlight.
const MAX_FILL: f32 = 0.4;

/// Each room's fill (DESIGN.md §6e): a closed room returns its lights' flux
/// from its walls, `BOUNCE · Σ(strength · reach²) / area`, flat over its
/// floor. Per cell it covers: the cell's index and its value in each
/// channel, stored as the bake stores light, at `FIRE_SCALE`.
fn fill(w: &World, lamps: &[Lamp]) -> Vec<(usize, [u8; 4])> {
    let m = &w.map;
    let mut rooms: std::collections::BTreeMap<u32, [f32; 4]> = Default::default();
    for l in lamps {
        let p = rim_sim::IVec::new(l.x.floor() as i32, l.y.floor() as i32);
        let Some(room) = m.room_at(p).filter(|r| r.enclosed()) else { continue };
        rooms.entry(room.id).or_default()[l.channel] +=
            BOUNCE * l.strength * l.reach * l.reach / room.cells.max(1) as f32;
    }
    if rooms.is_empty() {
        return Vec::new();
    }
    (0..(m.w * m.h) as usize)
        .filter_map(|i| {
            let f = rooms.get(&m.room_ids(i).0)?;
            Some((i, f.map(|v| (v.min(MAX_FILL) * FIRE_SCALE * 255.0).round() as u8)))
        })
        .collect()
}

/// The fill's quads, `texels` a cell: one a cell, its value in its colour
/// and no centre (reach 0), so the bake shader adds it as it is.
fn fill_meshes(m: &Map, fill: &[(usize, [u8; 4])], texels: f32) -> Vec<Mesh> {
    fill.chunks(LAMPS_PER_MESH)
        .map(|chunk| {
            let mut vertices = Vec::with_capacity(chunk.len() * 4);
            let mut indices = Vec::with_capacity(chunk.len() * 6);
            for &(i, color) in chunk {
                let p = m.pos(i);
                let base = vertices.len() as u16;
                for (dx, dy) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
                    let position = vec3((p.x as f32 + dx) * texels, (p.y as f32 + dy) * texels, 0.0);
                    vertices.push(Vertex { position, uv: Vec2::ZERO, color, normal: Vec4::ZERO });
                }
                indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            }
            Mesh { vertices, indices, texture: None }
        })
        .collect()
}

/// What the sun pass last worked out, so a still sky isn't worked out again.
#[derive(Clone, Copy, Debug, PartialEq)]
enum SunKey {
    /// Below the horizon: nothing is lit, whatever the cloud or the walls.
    Down,
    Up {
        /// Azimuth and elevation, in steps of `step` thousandths of a degree.
        az: i64,
        elev: i64,
        step: i64,
        /// Penumbra growth, in fiftieths.
        soft: i64,
        /// Steps the march takes: a preset changed mid-game redoes it.
        steps: u32,
        occluders: u64,
    },
}

impl SunKey {
    /// The sun at `(azimuth, elevation)`, rounded to `step` degrees, marched
    /// `steps` times.
    fn new(sun: Option<(f64, f64)>, soft: f32, occluders: u64, step: f64, steps: u32) -> SunKey {
        let step = ((step * 1000.0).round() as i64).max(1);
        let q = |x: f64| (x * 1000.0 / step as f64).round() as i64;
        match sun {
            Some((az, elev)) if q(elev) > 0 => {
                SunKey::Up { az: q(az), elev: q(elev), step, soft: (soft * 50.0).round() as i64, steps, occluders }
            }
            _ => SunKey::Down,
        }
    }

    /// Toward the sun, in cells, and tan(elevation); zero when it's down.
    fn toward(self) -> Vec3 {
        match self {
            SunKey::Down => Vec3::ZERO,
            SunKey::Up { az, elev, step, .. } => {
                let deg = |k: i64| (k * step) as f64 / 1000.0;
                let (az, elev) = (deg(az).to_radians(), deg(elev).to_radians());
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
    /// Firelight, baked: one flicker channel per colour channel, `texels`
    /// per cell, stored at 0.6 of its brightness so overlapping fires can
    /// add up past full before the texture clips (the shaders' SCALE).
    fires: Option<RenderTarget>,
    /// The field revision, occluders and size last looked at, so a frame
    /// in which nothing changed doesn't even list the lights.
    fires_seen: Option<(u64, u64)>,
    /// The lights `fires` was baked from.
    baked: Vec<Lamp>,
    /// Lights in the last bake, and the draws it took.
    lamps: usize,
    draws: u32,
    /// The room fill in `fires`, as data and as the quads that draw it.
    fill: Vec<(usize, [u8; 4])>,
    fill_meshes: Vec<Mesh>,
    /// Each roofed cell's share of the sky, R: what gets in through its
    /// room's walls and windows.
    rooms: Option<Texture2D>,
    /// The room rebuild and each room's share, 0 to 255, `rooms` is for.
    rooms_key: Option<(u64, Vec<u8>)>,
    /// How far the eye has adapted, 1 by day; eased toward what the sky asks.
    exposure: f32,
    /// Times firelight has been baked, whole or in part.
    pub bakes: u64,
    /// Where the sun reaches, R, `texels` per cell.
    sunlit: Option<RenderTarget>,
    sun_key: Option<SunKey>,
    /// `sunlit` holds a lightning flash's shadows, not the sun's.
    bolt: bool,
    materials: [Option<Material>; 4],
    /// Which shaders failed to build, by pass: without the sun's, the world
    /// is lit without sun shadows; without the bake, without firelight;
    /// without the multiply, unlit.
    failed: [bool; 4],
    /// Nothing, for a pass that couldn't run: no sun, no fire.
    blank: Option<Texture2D>,
    /// The player's lighting setting.
    pub setting: Setting,
    /// Light texels per cell now: the setting's, fewer when zoomed out.
    texels: u32,
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
    /// Lighting to the player's setting.
    pub fn with(setting: Setting) -> Light {
        Light { setting, ..Default::default() }
    }

    /// Bring every cached result up to date. Call before the world is drawn,
    /// with the default camera: it draws into targets of its own.
    /// `roofs` is each cell's roof, in steps in from its eaves
    /// (`Roofs::height`), so a house shades by its roof's shape.
    pub fn prepare(&mut self, w: &World, air: &Air, px_per_cell: f32, roofs: &[u8], flash: Flash) {
        self.passes.clear();
        // Zooming out drops texels, so the light never costs more than the
        // pixels it covers; a new size rebuilds every target.
        self.texels = texels_for(self.setting.quality.texels, px_per_cell, self.texels.max(1));
        let t = self.pass_begin();
        let changed = self.occluders.update(w, roofs);
        let rooms = self.update_rooms(w);
        // Uploads, not draws.
        self.pass_end("occluders", t, changed || rooms, 0);
        let t = self.pass_begin();
        let baked = self.bake_fires(w);
        let draws = if baked { self.draws } else { 0 };
        self.pass_end("firelight", t, baked, draws);
        let t = self.pass_begin();
        let ran = self.update_sun(w, air, flash);
        self.pass_end("sun", t, ran, ran as u32);
    }

    /// Multiply the world by the light. Call after everything lit is drawn.
    pub fn multiply(&mut self, w: &World, cam: &Cam, air: &Air, flash: Flash) {
        let t = self.pass_begin();
        let drew = self.draw_multiply(w, cam, air, flash);
        self.pass_end("multiply", t, drew, drew as u32);
    }

    /// Forget every cached result, so the next frame rebuilds them all: what
    /// the render bench times as the cost of a change.
    pub fn invalidate(&mut self) {
        self.fires_seen = None;
        self.rooms_key = None;
        self.sun_key = None;
        self.occluders.invalidate();
    }

    /// Let the eye adapt at once, on the next frame, instead of over a
    /// second: for tests that jump the light, whose checks would otherwise
    /// depend on how long the light before lasted.
    pub fn adapt_now(&mut self) {
        self.exposure = 0.0;
    }

    /// Whether the sun target holds a lightning flash's shadows now.
    pub fn lit_by_flash(&self) -> bool {
        self.bolt
    }

    /// The sun's visibility at a point, in cells: the light texel it falls
    /// in, 0 to 1, read back from the GPU. Slow, for tests.
    pub fn sun_visibility(&self, x: f32, y: f32) -> Option<f32> {
        self.sun_in(&self.sun_image()?, x, y)
    }

    /// The sun's visibility at a point in cells, from `sun_image`.
    pub fn sun_in(&self, img: &Image, x: f32, y: f32) -> Option<f32> {
        self.texel(img, x, y).map(|c| c.r)
    }

    /// The light texel a point in cells falls in, from a target read back.
    fn texel(&self, img: &Image, x: f32, y: f32) -> Option<Color> {
        let t = self.texels as f32;
        let (px, py) = ((x * t).floor(), (y * t).floor());
        (px >= 0.0 && py >= 0.0 && px < img.width as f32 && py < img.height as f32)
            .then(|| img.get_pixel(px as u32, py as u32))
    }

    /// Where the sun reaches, read back from the GPU, for `sun_in`. Slow,
    /// for tests that look at many points.
    pub fn sun_image(&self) -> Option<Image> {
        Some(self.sunlit.as_ref()?.texture.get_texture_data())
    }

    /// The baked firelight at a point, in cells: each channel's brightness,
    /// read back from the GPU. Slow, for tests.
    pub fn fire_at(&self, x: f32, y: f32) -> Option<[f32; 4]> {
        let c = self.texel(&self.fires.as_ref()?.texture.get_texture_data(), x, y)?;
        Some([c.r, c.g, c.b, c.a].map(|v| v / FIRE_SCALE))
    }

    /// Where the sun is now: pinned, or on the sky's path.
    fn sun(&self, w: &World) -> Option<(f64, f64)> {
        self.pin_sun.or_else(|| w.defs.sky.sun.as_ref().map(|p| sun_at(p, w.hour())))
    }

    fn material(&mut self, pass: Pass) -> Option<Material> {
        if self.failed[pass as usize] {
            return None;
        }
        let slot = &mut self.materials[pass as usize];
        if slot.is_none() {
            let (vertex, fragment, uniforms, textures, pipeline_params) = match pass {
                Pass::Sun => (
                    VERTEX,
                    SUN_FRAGMENT,
                    vec![
                        UniformDesc::new("map", UniformType::Float2),
                        UniformDesc::new("res", UniformType::Float2),
                        UniformDesc::new("sun", UniformType::Float4),
                        UniformDesc::new("steps", UniformType::Float1),
                    ],
                    vec!["occluders".to_string()],
                    PipelineParams { color_blend: None, ..Default::default() },
                ),
                Pass::Bake => {
                    // Lights add up, in every channel including alpha.
                    let add = BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::One);
                    (
                        BAKE_VERTEX,
                        BAKE_FRAGMENT,
                        vec![
                            UniformDesc::new("map", UniformType::Float2),
                            UniformDesc::new("res", UniformType::Float2),
                            UniformDesc::new("scale", UniformType::Float1),
                        ],
                        vec!["occluders".to_string()],
                        PipelineParams { color_blend: Some(add), alpha_blend: Some(add), ..Default::default() },
                    )
                }
                Pass::Clear => (
                    VERTEX,
                    CLEAR_FRAGMENT,
                    Vec::new(),
                    Vec::new(),
                    PipelineParams { color_blend: None, alpha_blend: None, ..Default::default() },
                ),
                Pass::Multiply => (
                    VERTEX,
                    MULTIPLY_FRAGMENT,
                    vec![
                        UniformDesc::new("ambient", UniformType::Float3),
                        UniformDesc::new("direct", UniformType::Float3),
                        UniformDesc::new("bolt", UniformType::Float3),
                        UniformDesc::new("night", UniformType::Float3),
                        UniformDesc::new("ch0", UniformType::Float3),
                        UniformDesc::new("ch1", UniformType::Float3),
                        UniformDesc::new("ch2", UniformType::Float3),
                        UniformDesc::new("ch3", UniformType::Float3),
                        UniformDesc::new("scale", UniformType::Float1),
                        UniformDesc::new("cell", UniformType::Float2),
                        UniformDesc::new("lres", UniformType::Float2),
                        UniformDesc::new("day", UniformType::Float1),
                        UniformDesc::new("exposure", UniformType::Float1),
                    ],
                    vec!["occluders".to_string(), "sunlit".to_string(), "rooms".to_string()],
                    PipelineParams {
                        // Multiply: result = source × destination.
                        color_blend: Some(BlendState::new(
                            Equation::Add,
                            BlendFactor::Value(BlendValue::DestinationColor),
                            BlendFactor::Zero,
                        )),
                        ..Default::default()
                    },
                ),
            };
            let m = load_material(
                ShaderSource::Glsl { vertex, fragment },
                MaterialParams { uniforms, textures, pipeline_params },
            );
            match m {
                Ok(m) => *slot = Some(m),
                Err(e) => {
                    let without = match pass {
                        Pass::Sun => "without sun shadows",
                        Pass::Bake => "without firelight",
                        Pass::Multiply => "unlit",
                        Pass::Clear => "with firelight rebaked whole",
                    };
                    eprintln!("lighting shader failed, drawing {without}: {e}");
                    self.failed[pass as usize] = true;
                    return None;
                }
            }
        }
        slot.clone()
    }

    /// Bake every light-giving thing's glow, if a light or the occluders
    /// changed. Whether it did.
    fn bake_fires(&mut self, w: &World) -> bool {
        let Some(occ) = self.occluders.texture.clone() else { return false };
        let size = (w.map.w as u32 * self.texels, w.map.h as u32 * self.texels);
        let seen = (w.fields.revision, self.occluders.version);
        let fits = self.fires.as_ref().is_some_and(|t| (t.texture.width() as u32, t.texture.height() as u32) == size);
        if fits && self.fires_seen == Some(seen) {
            return false;
        }
        let walls_moved = self.fires_seen.is_none_or(|(_, v)| v != self.occluders.version);
        self.fires_seen = Some(seen);
        let lamps = lamps(w);
        let walls = walls_moved.then(|| if self.occluders.whole { &[][..] } else { &self.occluders.changed[..] });
        // A heater re-stamped, or a wall went up where no light reaches: the
        // glow is the same, and nothing is redone. A room's fill spreads a
        // light over its whole floor, so a change to any fill redoes
        // everything; a room rebuild that leaves every fill as it was
        // doesn't.
        let fill = fill(w, &lamps);
        let work = if fits && fill == self.fill { redo(&self.baked, &lamps, walls) } else { Redo::Whole };
        if work == Redo::Nothing {
            return false;
        }
        let (Some(m), Some(clear)) = (self.material(Pass::Bake), self.material(Pass::Clear)) else { return false };
        if !fits {
            let rt = render_target(size.0, size.1);
            rt.texture.set_filter(FilterMode::Linear);
            self.fires = Some(rt);
        }
        let (tw, th) = (size.0 as f32, size.1 as f32);
        let t = self.texels as f32;
        set_camera(&Camera2D {
            zoom: vec2(2.0 / tw, 2.0 / th),
            target: vec2(tw / 2.0, th / 2.0),
            render_target: self.fires.clone(),
            ..Default::default()
        });
        m.set_texture("occluders", occ);
        m.set_uniform("map", vec2(w.map.w as f32, w.map.h as f32));
        m.set_uniform("res", vec2(tw, th));
        m.set_uniform("scale", FIRE_SCALE);
        self.draws = 0;
        match work {
            Redo::Nothing => {}
            Redo::Whole => {
                self.fill_meshes = fill_meshes(&w.map, &fill, t);
                self.fill = fill;
                clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
                gl_use_material(&m);
                for mesh in lamp_meshes(&lamps, t).iter().chain(&self.fill_meshes) {
                    draw_mesh(mesh);
                    self.draws += 1;
                }
            }
            Redo::Areas(areas) => {
                // Each area cleared and redrawn from every light that reaches
                // it, and the room fill it holds, under a scissor so lights
                // reaching past it aren't added twice outside it.
                for a in areas {
                    let (x0, y0) = ((a[0] * t).floor().max(0.0), (a[1] * t).floor().max(0.0));
                    let (x1, y1) = ((a[2] * t).ceil().min(tw), (a[3] * t).ceil().min(th));
                    if x1 <= x0 || y1 <= y0 {
                        continue;
                    }
                    let inside: Vec<Lamp> = lamps.iter().filter(|l| l.touches(a)).copied().collect();
                    // macroquad's scissor counts rows from the top; the
                    // target's rows run up from the bottom.
                    let clip = (x0 as i32, (th - y1) as i32, (x1 - x0) as i32, (y1 - y0) as i32);
                    // SAFETY: on the render thread, between draws.
                    unsafe { get_internal_gl() }.quad_gl.scissor(Some(clip));
                    gl_use_material(&clear);
                    draw_rectangle(x0, y0, x1 - x0, y1 - y0, WHITE);
                    gl_use_material(&m);
                    for mesh in lamp_meshes(&inside, t).iter().chain(&self.fill_meshes) {
                        draw_mesh(mesh);
                    }
                    self.draws += 2;
                }
                // SAFETY: as above.
                unsafe { get_internal_gl() }.quad_gl.scissor(None);
            }
        }
        gl_use_default_material();
        set_default_camera();
        self.lamps = lamps.len();
        self.baked = lamps;
        self.bakes += 1;
        true
    }

    /// Each roofed cell's share of the sky: `[[sky]]`'s `indoor_share`, what
    /// gets through walls and doors, plus the pass its room's windows give
    /// the light field (DESIGN.md §6c). Checked every frame, since the field
    /// sums a room's windows after the map rebuilds it; a room at a time,
    /// so it's cheap. Whether it changed.
    fn update_rooms(&mut self, w: &World) -> bool {
        let m = &w.map;
        let light = w.defs.lookup("field", "light").map(|f| f as usize);
        let base = w.defs.sky.indoor_share;
        let shares = (1..=m.room_count() as u32)
            .map(|id| {
                if !m.room_by_id(id).enclosed() {
                    return 0;
                }
                let pass = light.map_or(0.0, |f| w.fields.boundary(f, id).1);
                ((base + pass).clamp(0.0, 1.0) * 255.0).round() as u8
            })
            .collect();
        let key = (m.room_rebuilds, shares);
        if self.rooms.is_some() && self.rooms_key.as_ref() == Some(&key) {
            return false;
        }
        let mut bytes = vec![0u8; (m.w * m.h * 4) as usize];
        for i in 0..(m.w * m.h) as usize {
            let id = m.room_ids(i).0;
            if id > 0 {
                bytes[i * 4] = key.1[id as usize - 1];
            }
        }
        self.rooms_key = Some(key);
        let img = Image { bytes, width: m.w as u16, height: m.h as u16 };
        match &self.rooms {
            Some(t) if t.width() as i32 == m.w && t.height() as i32 == m.h => t.update(&img),
            _ => {
                let t = Texture2D::from_image(&img);
                t.set_filter(FilterMode::Linear);
                self.rooms = Some(t);
            }
        }
        true
    }

    /// Work out where the sun reaches, if it moved or the occluders changed.
    /// Whether it did.
    fn update_sun(&mut self, w: &World, air: &Air, flash: Flash) -> bool {
        let Some(occ) = self.occluders.texture.clone() else { return false };
        let size = (w.map.w as u32 * self.texels, w.map.h as u32 * self.texels);
        let q = self.setting.quality;
        // A flash is a light of its own for its few frames: one key while it
        // lasts, hard-edged, from where the bolt is.
        let bolt = flash.strength > FLASH_ON;
        let key = if bolt {
            let from = Some((flash.azimuth as f64, FLASH_ELEVATION));
            SunKey::new(from, 0.0, self.occluders.version, q.sun_rebuild, q.sun_steps)
        } else {
            let soft = if q.soft { penumbra(air.cloud) } else { 0.0 };
            SunKey::new(self.sun(w), soft, self.occluders.version, q.sun_rebuild, q.sun_steps)
        };
        let fits = self.sunlit.as_ref().is_some_and(|t| (t.texture.width() as u32, t.texture.height() as u32) == size);
        if fits && self.sun_key == Some(key) {
            return false;
        }
        let Some(m) = self.material(Pass::Sun) else { return false };
        if !fits {
            let rt = render_target(size.0, size.1);
            rt.texture.set_filter(FilterMode::Linear);
            self.sunlit = Some(rt);
        }
        self.sun_key = Some(key);
        self.bolt = bolt;
        // The key's quantised sun, so the result is exactly the key's.
        let (toward, soft) = (key.toward(), key.soft());
        let (tw, th) = (size.0 as f32, size.1 as f32);
        m.set_texture("occluders", occ);
        m.set_uniform("map", vec2(w.map.w as f32, w.map.h as f32));
        m.set_uniform("res", vec2(tw, th));
        m.set_uniform("sun", toward.extend(soft));
        m.set_uniform("steps", q.sun_steps as f32);
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

    /// The light on something outdoors, above every shadow: the sky as the
    /// eye has adapted to it, never darker than night, as the multiply
    /// lights open ground. Roofs are drawn in it.
    pub fn outdoor(&self, w: &World, air: &Air, flash: Flash) -> Vec3 {
        let (lit, bolt) = self.flash_light(flash);
        ((Self::sky_color(w, air, lit) + bolt) * self.exposure.max(1.0)).max(rgb3(w.defs.sky.rgb_night))
    }

    /// A flash's light: how much it brightens the sky everywhere, and the
    /// bolt's own, which lands only where `sunlit` says. A flash struck
    /// since the sun pass last ran lights everything evenly for its frame.
    fn flash_light(&self, flash: Flash) -> (f32, Vec3) {
        if self.bolt {
            (flash.strength * FLASH_AMBIENT, FLASH_RGB * flash.strength * (1.0 - FLASH_AMBIENT))
        } else {
            (flash.strength, Vec3::ZERO)
        }
    }

    /// How each way a roof slopes is lit, as `roof_faces` says, for the sun
    /// now.
    pub fn roof_faces(&self, w: &World, air: &Air) -> [f32; 5] {
        let sun = self.sun(w);
        roof_faces(sun, direct_share(sun.map_or(-1.0, |s| s.1), air.cloud))
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
    fn draw_multiply(&mut self, w: &World, cam: &Cam, air: &Air, flash: Flash) -> bool {
        let Some(occ) = self.occluders.texture.clone() else { return false };
        // Without the sun pass the sun reaches nowhere; without the bake,
        // no fire does.
        let blank = self.blank.get_or_insert_with(|| Texture2D::from_rgba8(1, 1, &[0, 0, 0, 0])).clone();
        let fires = self.fires.as_ref().map_or(blank.clone(), |t| t.texture.clone());
        let sun = self.sunlit.as_ref().map_or(blank.clone(), |t| t.texture.clone());
        let rooms = self.rooms.clone().unwrap_or(blank);
        let Some(m) = self.material(Pass::Multiply) else { return false };
        // A flash whose shadows are worked out lights the cloud a little and
        // the rest from where it is; one struck since, not yet, lights all.
        let (lit, bolt) = self.flash_light(flash);
        let sky = Self::sky_color(w, air, lit);
        let elev = self.sun(w).map_or(-1.0, |s| s.1);
        // While `sunlit` holds the bolt's shadows the sun has none: its
        // light is the sky's for those few frames, not the bolt's pattern.
        let share = if self.bolt { 0.0 } else { direct_share(elev, air.cloud) };
        let def = &w.defs.sky;
        let (mw, mh) = (w.map.w as f32, w.map.h as f32);
        m.set_texture("occluders", occ);
        m.set_texture("sunlit", sun);
        m.set_texture("rooms", rooms);
        let target = exposure_for(sky);
        // A frame's worth of adapting, about a second to settle.
        self.exposure = if self.exposure > 0.0 {
            self.exposure + (target - self.exposure) * (get_frame_time() * 1.6).min(1.0)
        } else {
            target
        };
        m.set_uniform("exposure", self.exposure);
        m.set_uniform("ambient", sky * (1.0 - share));
        m.set_uniform("direct", sky * share);
        m.set_uniform("bolt", bolt);
        m.set_uniform("night", rgb3(def.rgb_night));
        let fire = rgb3(def.rgb_fire) * 1.2;
        for (k, name) in ["ch0", "ch1", "ch2", "ch3"].into_iter().enumerate() {
            m.set_uniform(name, channel_colour(fire, k, get_time()));
        }
        m.set_uniform("cell", vec2(1.0 / mw, 1.0 / mh));
        m.set_uniform("lres", vec2(mw, mh) * self.texels as f32);
        // With no sun path the contact shadow fades with daylight instead.
        let day = if def.sun.is_none() { (air.light / 100.0).clamp(0.0, 1.0) } else { 0.0 };
        m.set_uniform("day", day);
        m.set_uniform("scale", FIRE_SCALE);
        gl_use_material(&m);
        let (sx, sy) = cam.to_screen(0.0, 0.0);
        draw_texture_ex(
            &fires,
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
        assert_eq!(
            SunKey::new(Some((180.0, -5.0)), 0.18, 7, 0.25, 28),
            SunKey::new(None, 0.03, 9, 0.25, 28),
            "down is down"
        );
        assert_eq!(SunKey::new(Some((90.0, 0.1)), 0.03, 1, 0.25, 28), SunKey::Down, "under a quarter degree is down");
        let up = SunKey::new(Some((90.0, 30.0)), 0.03, 1, 0.25, 28);
        assert_eq!(up, SunKey::new(Some((90.1, 30.1)), 0.035, 1, 0.25, 28), "small moves and cloud drift reuse it");
        assert_ne!(up, SunKey::new(Some((90.3, 30.0)), 0.03, 1, 0.25, 28), "a quarter degree works it out again");
        assert_ne!(up, SunKey::new(Some((90.0, 30.0)), 0.03, 2, 0.25, 28), "so do changed walls");
        assert_ne!(up, SunKey::new(Some((90.0, 30.0)), 0.03, 1, 0.25, 40), "and a preset that marches further");
        assert_eq!(SunKey::Down.toward(), Vec3::ZERO);
    }

    #[test]
    fn fires_side_by_side_never_pulse_in_step() {
        for (x, y) in [(0, 0), (7, -3), (-40, 12)] {
            let here = channel_of(Flicker::Fire, x, y);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                assert_ne!(here, channel_of(Flicker::Fire, x + dx, y + dy), "({x},{y}) and its neighbour");
            }
            assert!(here < STEADY);
        }
        assert_eq!(channel_of(Flicker::Steady, 3, 4), STEADY);
    }

    #[test]
    fn a_bake_redoes_only_what_changed_near_the_lights() {
        let torch = |x: f32, y: f32| Lamp { x, y, reach: 3.0, strength: 1.0, channel: 0 };
        let (a, b) = (torch(10.5, 10.5), torch(40.5, 10.5));
        assert_eq!(redo(&[a, b], &[a, b], None), Redo::Nothing, "a heater re-stamped");
        let far = [(70, 70, 32, 32)];
        assert_eq!(redo(&[a, b], &[a, b], Some(&far)), Redo::Nothing, "a wall where no light reaches");
        let near = [(0, 0, 32, 32)];
        assert_eq!(redo(&[a, b], &[a, b], Some(&near)), Redo::Areas(vec![a.area()]), "a wall by one light");
        let c = torch(60.5, 60.5);
        assert_eq!(redo(&[a, b], &[a, b, c], None), Redo::Areas(vec![c.area()]), "a new light, only its own area");
        assert_eq!(redo(&[a, b], &[a], None), Redo::Areas(vec![b.area()]), "a light gone");
        assert_eq!(redo(&[a], &[a], Some(&[])), Redo::Whole, "every wall repacked");
    }

    #[test]
    fn a_flame_dances_smoothly_and_a_steady_light_holds() {
        let fire = vec3(1.0, 0.7, 0.4);
        let mut last = flicker(0.0, 3.1);
        for k in 1..2000 {
            let f = flicker(k as f64 / 600.0, 3.1);
            assert!((0.5..1.2).contains(&f), "{f} at {k}");
            assert!((f - last).abs() < 0.08, "no jumps between frames: {last} to {f}");
            last = f;
        }
        let a = channel_colour(fire, 0, 12.3);
        assert_ne!(a, channel_colour(fire, 1, 12.3), "channels flicker out of step");
        assert_eq!(channel_colour(fire, STEADY, 12.3), fire);
    }

    #[test]
    fn each_light_is_one_quad_carrying_its_centre_reach_and_channel() {
        let lamp = Lamp { x: 10.5, y: 4.5, reach: 7.0, strength: 0.8, channel: 2 };
        let meshes = lamp_meshes(&[lamp, Lamp { channel: STEADY, ..lamp }], 2.0);
        assert_eq!(meshes.len(), 1);
        let m = &meshes[0];
        assert_eq!((m.vertices.len(), m.indices.len()), (8, 12));
        assert_eq!(m.vertices[0].normal, vec4(10.5, 4.5, 7.0, 0.8));
        assert_eq!(m.vertices[0].color, [0, 0, 255, 0]);
        assert_eq!(m.vertices[4].color, [0, 0, 0, 255], "the steady channel is alpha");
        assert_eq!(m.vertices[0].position, vec3(3.0 * 2.0, -3.0 * 2.0, 0.0), "reach plus half a cell, in texels");
    }

    #[test]
    fn the_eye_opens_up_at_night_and_not_by_day() {
        assert_eq!(exposure_for(Vec3::ONE), 1.0, "a bright day");
        let night = exposure_for(vec3(0.05, 0.06, 0.1));
        assert!((2.0..=2.6).contains(&night), "{night}");
        assert!(exposure_for(vec3(0.3, 0.3, 0.3)) < night, "dusk is between");
    }

    #[test]
    fn a_roof_slope_facing_the_sun_is_the_bright_one() {
        let share = direct_share(20.0, 0.0);
        // Morning: the sun in the east (+x), so the east slope is lit.
        let morning = roof_faces(Some((10.0, 20.0)), share);
        assert!(morning[2] > morning[4] && morning[4] > morning[1], "east over flat over west: {morning:?}");
        // Evening, the other way round.
        let evening = roof_faces(Some((170.0, 20.0)), share);
        assert!(evening[1] > evening[4] && evening[4] > evening[2], "west over flat over east: {evening:?}");
        // Noon from the south (+y): the south slope.
        let noon = roof_faces(Some((90.0, 60.0)), direct_share(60.0, 0.0));
        assert!(noon[3] > noon[0], "{noon:?}");
        assert_eq!(noon[4], 1.0, "a flat roof is lit as the ground is");
        let night = roof_faces(Some((200.0, -5.0)), 0.0);
        assert_eq!(night, roof_faces(None, 0.0), "no sun, the old light from the north-west");
        // Sunset comes on gradually, and cloud keeps the fixed light's relief.
        let dusk = roof_faces(Some((180.0, 0.5)), direct_share(0.5, 0.0));
        assert!(dusk.iter().zip(night).all(|(a, b)| (a - b).abs() < 0.05), "{dusk:?} against {night:?}");
        let overcast = roof_faces(Some((90.0, 60.0)), direct_share(60.0, 100.0));
        assert!(overcast[0] - overcast[3] > 0.25, "slopes still differ under cloud: {overcast:?}");
        assert!(morning.iter().all(|&f| (0.0..=1.6).contains(&f)), "{morning:?}");
    }

    #[test]
    fn the_sun_gives_most_of_a_clear_day_and_little_under_cloud() {
        assert_eq!(direct_share(-5.0, 0.0), 0.0);
        assert!((direct_share(45.0, 0.0) - DIRECT).abs() < 1e-6);
        assert!(direct_share(45.0, 100.0) < DIRECT * 0.25);
        assert!(penumbra(100.0) > penumbra(0.0) * 4.0, "cloud softens shadows");
    }
}
