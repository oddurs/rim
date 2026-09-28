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
use crate::quality::{texels_for, Setting, Watch};
use crate::sky::{Air, Flash};
use crate::Cam;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;
use rim_sim::defs::{Flicker, SunPath};
use rim_sim::map::Map;
use rim_sim::world::{Thing, World};
use rim_sim::IVec;

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
/// Moving lights nearest `centre` first, and past the first `cap` of them,
/// their strength negated: the bake shader draws those without shadows.
fn cap_shadows(lights: &[Lamp], centre: Vec2, cap: usize) -> Vec<Lamp> {
    let mut lights = lights.to_vec();
    let near = |l: &Lamp| vec2(l.x, l.y).distance_squared(centre);
    lights.sort_by(|a, b| near(a).total_cmp(&near(b)));
    for l in lights.iter_mut().skip(cap) {
        l.strength = -l.strength;
    }
    lights
}

/// How much of the open sky each cell of level `z` sees, by plane index:
/// all of it on the surface and above. Below it, a cell with air over it to
/// the surface sees `width / (width + 2 · depth)` of it, the solid angle a
/// shaft leaves open from its bottom, its width the narrower of its runs
/// across; a cell under rock sees none.
fn open_sky(m: &Map, z: i32) -> Vec<f32> {
    let n = (m.w * m.h) as usize;
    if z >= 0 {
        return vec![1.0; n];
    }
    let open: Vec<bool> = (0..n).map(|i| crate::occluders::open_to_sky(m, i as i32 % m.w, i as i32 / m.w, z)).collect();
    let run = |i: usize, (dx, dy): (i32, i32)| {
        let (x, y) = (i as i32 % m.w, i as i32 / m.w);
        let mut k = 1;
        while {
            let (a, b) = (x + dx * k, y + dy * k);
            a >= 0 && b >= 0 && a < m.w && b < m.h && open[(b * m.w + a) as usize]
        } {
            k += 1;
        }
        k - 1
    };
    let depth = -z as f32;
    (0..n)
        .map(|i| {
            if !open[i] {
                return 0.0;
            }
            let across = 1 + run(i, (1, 0)) + run(i, (-1, 0));
            let down = 1 + run(i, (0, 1)) + run(i, (0, -1));
            let width = across.min(down) as f32;
            width / (width + 2.0 * depth)
        })
        .collect()
}

/// How much of night's light a level below the surface keeps: enough to make
/// out the rock, no more.
const UNDERGROUND: f32 = 0.25;

/// The least time between firelight bakes, in seconds. A light that comes
/// sooner is drawn as a moving light until the next.
const SETTLE: f64 = 1.0;
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
uniform vec4 sun0;
uniform vec4 sun1;
uniform vec4 sun2;
uniform vec4 sun3;
uniform float count;
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
// How much of a body at `sun` (toward it in cells, tan elevation, penumbra
// growth) reaches `p`, whose occluder is `here`.
float march(vec2 p, vec4 here, vec4 sun) {
    if (sun.z <= 0.0) return 0.0;
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
    return vis;
}
// Each shadowed sky body in a channel of its own, brightest in red.
void main() {
    vec2 p = gl_FragCoord.xy / res * map;
    vec4 here = cell(p);
    // Rock below the surface with more over it: its top is not the sky's.
    if (kind(here) == 3.0) {
        gl_FragColor = vec4(0.0);
        return;
    }
    vec4 vis = vec4(0.0);
    if (count > 0.5) vis.r = march(p, here, sun0);
    if (count > 1.5) vis.g = march(p, here, sun1);
    if (count > 2.5) vis.b = march(p, here, sun2);
    if (count > 3.5) vis.a = march(p, here, sun3);
    gl_FragColor = vis;
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
    // A light past the moving lights' cap comes with its strength negated:
    // it glows, and casts no shadow.
    float lit = 8.0;
    if (lamp.w > 0.0) {
        lit = 0.0;
        for (int r = 0; r < 8; r++) {
            lit += trace(p, lamp.xy + across * (float(r) / 7.0 - 0.5) * 2.0 * FLAME);
        }
    }
    gl_FragColor = channel * (abs(lamp.w) * fall * lit / 8.0 * scale);
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
uniform sampler2D moving;
uniform sampler2D occluders;
uniform sampler2D sunlit;
uniform sampler2D rooms;
uniform float exposure;
uniform vec3 ambient;
uniform vec3 direct0;
uniform vec3 direct1;
uniform vec3 direct2;
uniform vec3 direct3;
uniform vec4 weights;
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
    vec4 vis = mix(texture2D(sunlit, uv), texture2D(sunlit, centre), solid);
    // Indoors, a room's share of the sky through its walls and windows,
    // and the sun itself where it comes through a pane: from this texel
    // alone, so the linear filter doesn't carry the sun on the wall's
    // outer face onto the floor inside.
    float share = texture2D(rooms, uv).r;
    float open = texture2D(rooms, uv).g;
    vec4 vis_in = texture2D(sunlit, (floor(uv * lres) + 0.5) / lres);
    // Each shadowed sky body in its channel, in its colour. A lightning
    // flash, while it lasts, is what red holds.
    vec3 lit = direct0 * vis.r + direct1 * vis.g + direct2 * vis.b + direct3 * vis.a + bolt * vis.r;
    vec3 lit_in = direct0 * vis_in.r + direct1 * vis_in.g + direct2 * vis_in.b + direct3 * vis_in.a + bolt * vis_in.r;
    vec3 outside = ambient * open + lit;
    vec3 inside = (ambient + direct0 + direct1 + direct2 + direct3) * share + lit_in;
    // How much of the direct light reaches here, for the contact shadow.
    float sun = dot(vis, weights);
    // The baked firelight, and what the bake hasn't caught up with yet.
    vec4 f = (texture2D(Texture, uv) + texture2D(moving, uv)) / scale;
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

/// A GPU timer, made the first time it's asked for.
#[derive(Default)]
enum Timer<T> {
    #[default]
    Untried,
    /// GL has no timer, or none that can time a pass.
    Unavailable,
    Ready(T),
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
        if !gl_times_a_pass() {
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

/// Frames a lighting time is read back after: by then the GPU has long run
/// them, so reading never waits.
const COST_FRAMES: usize = 4;

/// What `auto` reads: the lighting's GPU time a frame, from timer queries
/// read back `COST_FRAMES` later. A frame has two: the passes before the
/// world is drawn, and the multiply after it.
struct CostTimer {
    ids: [[u32; 2]; COST_FRAMES],
    /// Per frame in flight: which of its queries ran, and what it was timed
    /// under (`Light::cost_tag`).
    ran: [([bool; 2], (usize, u32)); COST_FRAMES],
    /// The frame being timed, whether this one is, and the query open in it.
    at: usize,
    timing: bool,
    open: bool,
}

impl CostTimer {
    fn new() -> Option<CostTimer> {
        if !gl_times_a_pass() {
            return None;
        }
        let mut ids = [[0; 2]; COST_FRAMES];
        // SAFETY: desktop GL 3.3 has query objects; on the render thread.
        unsafe { miniquad::gl::glGenQueries((2 * COST_FRAMES) as i32, ids.as_mut_ptr().cast()) };
        let ran = [([false; 2], (0, 0)); COST_FRAMES];
        let t = CostTimer { ids, ran, at: 0, timing: false, open: false };
        ids.iter().flatten().all(|&id| id != 0).then_some(t)
    }

    /// Start a frame, timed under `tag`, in the oldest frame's queries: what
    /// that frame was timed under and its GPU time, µs, if both its queries
    /// ran. A GPU still running it holds its queries, and this frame goes
    /// untimed rather than restart one the GPU hasn't finished.
    fn frame(&mut self, tag: (usize, u32)) -> Option<((usize, u32), f64)> {
        use miniquad::gl::*;
        let next = (self.at + 1) % COST_FRAMES;
        let (ran, was) = self.ran[next];
        let mut ns = [0 as GLuint64; 2];
        for ((&id, ns), _) in self.ids[next].iter().zip(&mut ns).zip(ran).filter(|(_, r)| *r) {
            let mut ready: GLint = 0;
            // SAFETY: a query that has ended; asking whether it's ready, and
            // reading it once it is, doesn't wait.
            unsafe {
                glGetQueryObjectiv(id, GL_QUERY_RESULT_AVAILABLE, &mut ready);
                if ready == 0 {
                    self.timing = false;
                    return None;
                }
                glGetQueryObjectui64v(id, GL_QUERY_RESULT, ns);
            }
        }
        (self.at, self.timing) = (next, true);
        self.ran[next] = ([false; 2], tag);
        (ran == [true; 2]).then(|| (was, (ns[0] + ns[1]) as f64 / 1e3))
    }

    /// Time query `k` of this frame, until `end`.
    fn begin(&mut self, k: usize) {
        if !self.timing {
            return;
        }
        flush_batches();
        // SAFETY: a query `new` made whose last result has been read, and
        // none other running: `end` closes each.
        unsafe { miniquad::gl::glBeginQuery(miniquad::gl::GL_TIME_ELAPSED, self.ids[self.at][k]) };
        self.ran[self.at].0[k] = true;
        self.open = true;
    }

    fn end(&mut self) {
        if std::mem::take(&mut self.open) {
            flush_batches();
            // SAFETY: ends the query `begin` started, without waiting for it.
            unsafe { miniquad::gl::glEndQuery(miniquad::gl::GL_TIME_ELAPSED) };
        }
    }
}

impl Drop for CostTimer {
    fn drop(&mut self) {
        // SAFETY: the queries `new` made, deleted once.
        unsafe { miniquad::gl::glDeleteQueries((2 * COST_FRAMES) as i32, self.ids.as_ptr().cast()) };
    }
}

/// Whether this context's GL can time one pass.
fn gl_times_a_pass() -> bool {
    times_a_pass(&gl_string(miniquad::gl::GL_VERSION), &gl_renderer())
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

/// A sky body as it lights the world now (DESIGN.md §6e).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    /// Which it is: its index among the sky's bodies, or `LONE_SUN`.
    pub id: usize,
    /// Azimuth and elevation, degrees.
    pub at: (f64, f64),
    /// Its share of the sky's light, 0 to 1: its term over its field's value.
    pub share: f32,
    /// Its colour, white at most.
    pub rgb: Vec3,
    /// How much wider than the sun it looks: its shadows soften as much more.
    pub size: f32,
    pub shadows: bool,
}

impl Body {
    /// How much of the sky's light comes straight from it.
    fn direct(&self, cloud: f32) -> f32 {
        self.share * direct_share(self.at.1, cloud)
    }
}

/// The `Body::id` of a sky's lone `sun`, where it declares no bodies.
pub const LONE_SUN: usize = usize::MAX;

/// Light, in the `light` field's units, at which a body's reach clears the
/// plan's contact shadow wholly: daylight does, moonlight barely.
const CONTACT_LIGHT: f32 = 10.0;

/// Under this, in its field's units, a sky body gives no light.
const DARK_BODY: f64 = 0.01;

/// Which bodies cast shadows: those that may, above the horizon, brightest
/// straight light first, `cap` at most. The rest light without.
fn shadow_slots(bodies: &[Body], cloud: f32, cap: usize) -> Vec<usize> {
    // Clear of the horizon, or its shadows round to none.
    let mut up: Vec<&Body> = bodies.iter().filter(|b| b.shadows && b.at.1 > 0.5 && b.direct(cloud) > 1e-3).collect();
    up.sort_by(|a, b| b.direct(cloud).total_cmp(&a.direct(cloud)).then(a.id.cmp(&b.id)));
    up.into_iter().take(cap.min(4)).map(|b| b.id).collect()
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
fn lamps(w: &World, z: i32) -> Vec<Lamp> {
    let Some(field) = w.defs.lookup("field", "light") else { return Vec::new() };
    w.fields
        .emitters_of(field as usize)
        .filter(|&(_, p, amount, _)| amount > 0.0 && p.z == z)
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

/// How much of a light a level passes on to the next through an opening.
const ACROSS: f32 = 0.5;

/// The light that reaches level `z` from the levels beside it, through the
/// openings between: the air over it and under it, and its stairs and
/// ladders. Each light on the next level up or down that reaches an opening
/// gives a light there, on `z`: as bright as it still is at the opening,
/// less `ACROSS` for the level it crosses, reaching as far as it has left to
/// go. `z`'s own walls then shade it, so a torch at the head of a stair
/// lights the steps below and fades out from the stairwell.
fn lamps_across(w: &World, z: i32) -> Vec<Lamp> {
    let m = &w.map;
    // Where light crosses to z, and from which level: the cells of z it
    // arrives in.
    let mut openings: Vec<(IVec, i32)> = Vec::new();
    // Air on the level above opens onto z; z's own air opens onto the level below.
    openings.extend(m.air_cells(z + 1).iter().map(|&i| (m.pos(i as usize), z + 1)));
    openings.extend(m.air_cells(z).iter().map(|&i| (m.pos(i as usize), z - 1)));
    for p in m.portals() {
        if p.bottom.z == z && p.top.z == z + 1 {
            openings.push((p.bottom, z + 1));
        } else if p.top.z == z && p.bottom.z == z - 1 {
            openings.push((p.top, z - 1));
        }
    }
    if openings.is_empty() {
        return Vec::new();
    }
    let (above, below) = (lamps(w, z + 1), lamps(w, z - 1));
    let light = w.defs.lookup("field", "light");
    let mut out = Vec::new();
    for (from, lights) in [(z + 1, &above), (z - 1, &below)] {
        for l in lights {
            // What of this light reaches each opening onto z from its level.
            let mut through = Vec::new();
            for &(o, level) in &openings {
                if level != from {
                    continue;
                }
                let (ox, oy) = (o.x as f32 + 0.5, o.y as f32 + 0.5);
                let d = ((l.x - ox).powi(2) + (l.y - oy).powi(2)).sqrt();
                if d >= l.reach || !in_sight(w, light, (l.x, l.y), (ox, oy), from) {
                    continue;
                }
                // The bake's falloff, as the shader has it.
                let x2 = (d / l.reach).powi(2);
                let fall = (1.0 - x2 * x2).powi(2) / (1.0 + 0.08 * d * d);
                through.push(Lamp {
                    x: ox,
                    y: oy,
                    reach: (l.reach - d).max(0.75),
                    strength: l.strength * fall,
                    channel: l.channel,
                });
            }
            // A wide opening is many lights, which add up where they meet:
            // together they give no more than the brightest of them, so a
            // pit isn't brighter below than the light is above.
            let (sum, most) = through.iter().fold((0.0f32, 0.0f32), |(s, m), t| (s + t.strength, m.max(t.strength)));
            let share = if sum > 0.0 { most / sum } else { 0.0 };
            out.extend(
                through
                    .into_iter()
                    .map(|t| Lamp { strength: t.strength * share * ACROSS, ..t })
                    .filter(|t| t.strength >= 0.01),
            );
        }
    }
    out
}

/// Whether light from `a` reaches `b` on level `z`, both in cells: no wall
/// or door stands between them there. Half a cell a step.
fn in_sight(w: &World, light: Option<rim_sim::defs::DefId>, a: (f32, f32), b: (f32, f32), z: i32) -> bool {
    let m = &w.map;
    let d = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
    let n = (d * 2.0).ceil() as i32;
    (1..n).all(|k| {
        let t = k as f32 / n as f32;
        let p = IVec::at((a.0 + (b.0 - a.0) * t).floor() as i32, (a.1 + (b.1 - a.1) * t).floor() as i32, z);
        // The light's own cell and the opening's don't shade it.
        let (from, to) = ((a.0.floor() as i32, a.1.floor() as i32), (b.0.floor() as i32, b.1.floor() as i32));
        (p.x, p.y) == from
            || (p.x, p.y) == to
            || !m.inb(p)
            || !matches!(
                crate::occluders::occluder_at(w, m.idx(p), light),
                crate::occluders::Occluder::Solid { .. } | crate::occluders::Occluder::Door { .. }
            )
    })
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
fn fill(w: &World, lamps: &[Lamp], z: i32) -> Vec<(usize, [u8; 4])> {
    let m = &w.map;
    let mut rooms: std::collections::BTreeMap<u32, [f32; 4]> = Default::default();
    for l in lamps {
        let p = rim_sim::IVec::at(l.x.floor() as i32, l.y.floor() as i32, z);
        let Some(room) = m.room_at(p).filter(|r| r.enclosed()) else { continue };
        rooms.entry(room.id).or_default()[l.channel] +=
            BOUNCE * l.strength * l.reach * l.reach / room.cells.max(1) as f32;
    }
    if rooms.is_empty() {
        return Vec::new();
    }
    (0..(m.w * m.h) as usize)
        .filter_map(|i| {
            let p = m.pos(i);
            let f = rooms.get(&m.room_ids(m.idx(rim_sim::IVec::at(p.x, p.y, z))).0)?;
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

/// What the light keeps for one level: its occluders, its firelight, its
/// sun and its rooms.
#[derive(Default)]
struct Level {
    /// What stops light, per cell.
    occluders: Occluders,
    /// Firelight, baked: one flicker channel per colour channel, `texels`
    /// per cell, stored at 0.6 of its brightness so overlapping fires can
    /// add up past full before the texture clips (the shaders' SCALE).
    fires: Option<RenderTarget>,
    /// The field revision, occluders and size last looked at, so a frame
    /// in which nothing changed doesn't even list the lights.
    fires_seen: Option<(u64, u64, u64)>,
    /// The lights `fires` was baked from.
    baked: Vec<Lamp>,
    /// The lights the field has now. The bake catches up with them at most
    /// once a `SETTLE`; until it does, the ones it hasn't baked are drawn as
    /// moving lights.
    current: Vec<Lamp>,
    /// How many of `current` are the level's own; the rest come through
    /// openings from the levels beside.
    own: usize,
    /// A bake is owed, and the walls that changed since the last one: all
    /// of them with `owed_whole`.
    owed: bool,
    owed_walls: Vec<(i32, i32, i32, i32)>,
    owed_whole: bool,
    /// When the last bake ran, by the clock.
    last_bake: f64,
    /// Firelight the bake doesn't hold, drawn every frame: lights that came
    /// since the last bake, and moving ones.
    moving: Option<RenderTarget>,
    /// Whether `moving` holds any light now.
    moving_drawn: bool,
    /// Lights in the last bake, and the draws it took.
    lamps: usize,
    draws: u32,
    /// The room fill in `fires`, as data and as the quads that draw it.
    fill: Vec<(usize, [u8; 4])>,
    fill_meshes: Vec<Mesh>,
    /// Each roofed cell's share of the sky, R: what gets in through its
    /// room's walls and windows. And G, how much of the open sky each cell
    /// sees: all of it on the surface, less down a shaft, none under rock.
    rooms: Option<Texture2D>,
    /// The room rebuild, each room's share, 0 to 255, and the levels
    /// above's revisions, that `rooms` is for.
    rooms_key: Option<(u64, Vec<u8>, u64)>,
    /// Whether any cell of the level sees the open sky: below the surface,
    /// without a shaft the sun pass has nothing to do.
    any_sky: bool,
    /// The open sky each cell sees (`open_sky`), for the eye to adapt to.
    sky: Vec<f32>,
    /// Where each shadowed sky body reaches, a channel each, brightest in
    /// R, `texels` per cell.
    sunlit: Option<RenderTarget>,
    /// Whether it was a flash, each channel's body as it was marched, and
    /// which bodies they are.
    sun_key: Option<(bool, Vec<SunKey>, Vec<usize>)>,
    /// The bodies in `sunlit`'s channels, by `Body::id`.
    slots: Vec<usize>,
    /// `sunlit` holds a lightning flash's shadows, not the sun's.
    bolt: bool,
}

#[derive(Default)]
pub struct Light {
    /// Moving lights this frame (`set_moving`).
    carried: Vec<Lamp>,
    /// Lights drawn into `moving` last frame: with shadows, and in all.
    pub moving_lit: (usize, usize),
    /// How far the eye has adapted, 1 by day; eased toward what the sky asks.
    exposure: f32,
    /// Times firelight has been baked, whole or in part.
    pub bakes: u64,
    materials: [Option<Material>; 4],
    /// Which shaders failed to build, by pass: without the sun's, the world
    /// is lit without sun shadows; without the bake, without firelight;
    /// without the multiply, unlit.
    failed: [bool; 4],
    /// Nothing, for a pass that couldn't run: no sun, no fire.
    blank: Option<Texture2D>,
    /// The player's lighting setting.
    setting: Setting,
    /// Light texels per cell now: the setting's, fewer when zoomed out.
    texels: u32,
    /// Pin the sun: (azimuth, elevation), degrees. For tests and the bench.
    pub pin_sun: Option<(f64, f64)>,
    /// Times the sun pass has run.
    pub sun_runs: u64,
    /// The sky's bodies this frame, and the picture's light from the sky
    /// (`bodies`), worked out once in `prepare`.
    lit: Vec<Body>,
    sky_light: f32,
    /// Each lighting pass last frame, in order.
    pub passes: Vec<PassTime>,
    /// Time each pass on the GPU with a timer query (`time_gpu`).
    gpu_timing: bool,
    query: Timer<GpuTimer>,
    /// What `auto` times the lighting with, and what it has seen: the
    /// frames it has read, and which setting (a count of `set`s) they are
    /// for.
    cost: Timer<CostTimer>,
    watch: Watch,
    pub cost_frames: u64,
    chosen: u32,
    /// The level in view, and what the light keeps for it.
    z: i32,
    lv: Level,
    /// Levels kept beside it: those next to the one in view, so changing
    /// to one bakes nothing.
    cache: std::collections::BTreeMap<i32, Level>,
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
    /// `centre` is the middle of the view, in cells: moving lights nearest
    /// it cast shadows first.
    /// `z` is the level in view, lit by its own lights.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        w: &World,
        air: &Air,
        px_per_cell: f32,
        roofs: &[u8],
        flash: Flash,
        centre: Vec2,
        z: i32,
    ) {
        self.passes.clear();
        (self.lit, self.sky_light) = self.bodies(w, air.light);
        self.cost_frame();
        self.cost_begin(0);
        self.view(z);
        // Zooming out drops texels, so the light never costs more than the
        // pixels it covers; a new size rebuilds every target.
        self.texels = texels_for(self.setting.quality.texels, px_per_cell, self.texels.max(1));
        let t = self.pass_begin();
        // Roofs are the surface's; below and above it, a roof is its level's.
        let roofs = if z == 0 { roofs } else { &[] };
        let changed = self.lv.occluders.update(w, roofs, z);
        let rooms = self.update_rooms(w);
        // Uploads, not draws.
        self.pass_end("occluders", t, changed || rooms, 0);
        let t = self.pass_begin();
        let baked = self.bake_fires(w, get_time());
        let draws = if baked { self.lv.draws } else { 0 };
        self.pass_end("firelight", t, baked, draws);
        let t = self.pass_begin();
        let drew = self.draw_moving(w, centre);
        self.pass_end("moving", t, drew, drew as u32);
        let t = self.pass_begin();
        let ran = self.update_sun(w, air, flash);
        self.pass_end("sun", t, ran, ran as u32);
        self.cost_end();
    }

    /// The open sky over the cells in view, on average: how much of the
    /// sky's light the view holds. A sample of them, a few thousand at most.
    fn view_sky(&self, w: &World, cam: &Cam) -> f32 {
        let (m, sky) = (&w.map, &self.lv.sky);
        if sky.len() != (m.w * m.h) as usize {
            return 0.0;
        }
        let (a, b) = (cam.to_world(0.0, 0.0), cam.to_world(screen_width(), screen_height()));
        let (x0, y0) = ((a.0.floor() as i32).max(0), (a.1.floor() as i32).max(0));
        let (x1, y1) = ((b.0.ceil() as i32).min(m.w), (b.1.ceil() as i32).min(m.h));
        if x1 <= x0 || y1 <= y0 {
            return 0.0;
        }
        let step = (((x1 - x0) * (y1 - y0)) as f32 / 4096.0).sqrt().ceil().max(1.0) as usize;
        let (mut sum, mut n) = (0.0, 0);
        for y in (y0..y1).step_by(step) {
            for x in (x0..x1).step_by(step) {
                sum += sky[(y * m.w + x) as usize];
                n += 1;
            }
        }
        sum / n.max(1) as f32
    }

    /// Show level `z`: what the light keeps for the one in view goes to the
    /// cache, and what it kept for `z` comes back, or starts afresh. Levels
    /// more than one from `z` are let go, with their render targets.
    fn view(&mut self, z: i32) {
        if z != self.z {
            let was = std::mem::take(&mut self.lv);
            self.cache.insert(self.z, was);
            self.lv = self.cache.remove(&z).unwrap_or_default();
            self.z = z;
        }
        self.cache.retain(|&k, _| (k - z).abs() <= 1);
    }

    /// Multiply the world by the light. Call after everything lit is drawn.
    pub fn multiply(&mut self, w: &World, cam: &Cam, air: &Air, flash: Flash) {
        self.cost_begin(1);
        let t = self.pass_begin();
        let drew = self.draw_multiply(w, cam, air, flash);
        self.pass_end("multiply", t, drew, drew as u32);
        self.cost_end();
    }

    /// Forget every cached result, so the next frame rebuilds them all: what
    /// the render bench times as the cost of a change.
    pub fn invalidate(&mut self) {
        self.lv.fires_seen = None;
        self.lv.last_bake = f64::NEG_INFINITY;
        self.lv.rooms_key = None;
        self.lv.sun_key = None;
        self.lv.occluders.invalidate();
    }

    /// Let the eye adapt at once, on the next frame, instead of over a
    /// second: for tests that jump the light, whose checks would otherwise
    /// depend on how long the light before lasted.
    pub fn adapt_now(&mut self) {
        self.exposure = 0.0;
    }

    /// The sky over a point on the level in view, in cells: how much of
    /// the open sky it sees, and the sun's visibility there, read back from
    /// the GPU. Slow, for tests.
    pub fn sky_at(&self, x: f32, y: f32) -> Option<(f32, f32)> {
        let rooms = self.lv.rooms.as_ref()?.get_texture_data();
        let (cx, cy) = (x.floor() as u32, y.floor() as u32);
        let open = (cx < rooms.width as u32 && cy < rooms.height as u32).then(|| rooms.get_pixel(cx, cy).g)?;
        Some((open, self.sun_visibility(x, y)?))
    }

    /// The sky bodies casting shadows now, brightest first, by `Body::id`:
    /// an index into the sky's bodies, or `LONE_SUN`.
    pub fn shadow_bodies(&self) -> &[usize] {
        &self.lv.slots
    }

    /// Whether the sun target holds a lightning flash's shadows now.
    pub fn lit_by_flash(&self) -> bool {
        self.lv.bolt
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
        Some(self.lv.sunlit.as_ref()?.texture.get_texture_data())
    }

    /// The baked firelight at a point, in cells: each channel's brightness,
    /// read back from the GPU. Slow, for tests.
    pub fn fire_at(&self, x: f32, y: f32) -> Option<[f32; 4]> {
        let c = self.texel(&self.lv.fires.as_ref()?.texture.get_texture_data(), x, y)?;
        Some([c.r, c.g, c.b, c.a].map(|v| v / FIRE_SCALE))
    }

    /// The sky's bodies now, in the sky's order, and the light the picture
    /// has from the sky: the sim's `light`, and what the bodies that light
    /// only the picture add to it (core's moon). A body's share is its part
    /// of that. A sky with none has its lone `sun`, which is all its light.
    /// A pinned sun is all the sky's light too, from where it's pinned: the
    /// body with the most, moved.
    pub fn bodies(&self, w: &World, light: f32) -> (Vec<Body>, f32) {
        use rim_sim::defs::BodyLight;
        use rim_sim::terms::Terms;
        let (defs, hour) = (&w.defs, w.hour());
        let mut sky = light;
        let mut out: Vec<Body> = if defs.sky_bodies.is_empty() {
            let at = defs.sky.sun.as_ref().map(|p| sun_at(p, hour)).or(self.pin_sun);
            at.map(|at| Body { id: LONE_SUN, at, share: 1.0, rgb: Vec3::ONE, size: 1.0, shadows: true })
                .into_iter()
                .collect()
        } else {
            // A field's term is read against the field's terms themselves,
            // so a field pinned for a test or pushed by a plugin still says
            // whose light it is. A body all but dark (a new moon) is dark:
            // alone in the sky, a glimmer would be all its light, and cast.
            let eval = |terms: Vec<rim_sim::terms::Term>| w.fields.eval_global(&Terms { terms });
            let lights: Vec<f32> = defs
                .sky_bodies
                .iter()
                .map(|b| match &b.light {
                    BodyLight::Field { field, term } => {
                        let all = &defs.fields[*field].terms.terms;
                        let mine = eval(all.iter().filter(|t| &t.label == term).cloned().collect());
                        let total = eval(all.clone());
                        if mine > DARK_BODY {
                            (mine / total).clamp(0.0, 1.0) as f32 * light
                        } else {
                            0.0
                        }
                    }
                    BodyLight::Own(t) => {
                        let own = w.fields.eval_global(t) as f32;
                        if own > DARK_BODY as f32 {
                            own
                        } else {
                            0.0
                        }
                    }
                    BodyLight::Unresolved => 0.0,
                })
                .collect();
            sky += defs
                .sky_bodies
                .iter()
                .zip(&lights)
                .filter(|(b, _)| matches!(b.light, BodyLight::Own(_)))
                .map(|(_, l)| l)
                .sum::<f32>();
            defs.sky_bodies
                .iter()
                .zip(lights)
                .enumerate()
                .map(|(id, (b, l))| {
                    let share = if sky > 0.0 { (l / sky).clamp(0.0, 1.0) } else { 0.0 };
                    let size = (b.angular_size / 0.5) as f32;
                    Body { id, at: sun_at(&b.path(), hour), share, rgb: rgb3(b.rgb), size, shadows: b.shadows }
                })
                .collect()
        };
        if let Some(pin) = self.pin_sun {
            let most = out.iter().enumerate().max_by(|a, b| a.1.share.total_cmp(&b.1.share)).map(|(i, _)| i);
            for (i, b) in out.iter_mut().enumerate() {
                if Some(i) == most {
                    // A plain sun: white, the sun's size.
                    (b.at, b.share, b.rgb, b.size, b.shadows) = (pin, 1.0, Vec3::ONE, 1.0, true);
                } else {
                    b.share = 0.0;
                }
            }
        }
        (out, sky)
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
                        UniformDesc::new("sun0", UniformType::Float4),
                        UniformDesc::new("sun1", UniformType::Float4),
                        UniformDesc::new("sun2", UniformType::Float4),
                        UniformDesc::new("sun3", UniformType::Float4),
                        UniformDesc::new("count", UniformType::Float1),
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
                        UniformDesc::new("direct0", UniformType::Float3),
                        UniformDesc::new("direct1", UniformType::Float3),
                        UniformDesc::new("direct2", UniformType::Float3),
                        UniformDesc::new("direct3", UniformType::Float3),
                        UniformDesc::new("weights", UniformType::Float4),
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
                    vec!["occluders".to_string(), "sunlit".to_string(), "rooms".to_string(), "moving".to_string()],
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
    fn bake_fires(&mut self, w: &World, now: f64) -> bool {
        let Some(occ) = self.lv.occluders.texture.clone() else { return false };
        let size = (w.map.w as u32 * self.texels, w.map.h as u32 * self.texels);
        // The levels beside this one: their openings and lights reach it.
        let (m, z) = (&w.map, self.z);
        // This level and those beside it: their openings, and the lights
        // beside that reach through them.
        let beside = m.levels().filter(|&k| (k - z).abs() <= 1).map(|k| m.level_revision(k)).sum::<u64>();
        let seen = (w.fields.revision, self.lv.occluders.version, beside);
        let fits =
            self.lv.fires.as_ref().is_some_and(|t| (t.texture.width() as u32, t.texture.height() as u32) == size);
        if !fits || self.lv.fires_seen != Some(seen) {
            if self.lv.fires_seen.is_none_or(|(_, v, _)| v != self.lv.occluders.version) {
                self.lv.owed_whole |= self.lv.occluders.whole;
                self.lv.owed_walls.extend_from_slice(&self.lv.occluders.changed);
            }
            self.lv.fires_seen = Some(seen);
            self.lv.current = lamps(w, self.z);
            // The level's own lights first: they alone fill its rooms.
            self.lv.own = self.lv.current.len();
            self.lv.current.extend(lamps_across(w, self.z));
            self.lv.owed = true;
        }
        // At most one bake a SETTLE: a spreading fire's new flames are drawn
        // as moving lights meanwhile, rather than rebaking every frame.
        if !self.lv.owed || (fits && now - self.lv.last_bake < SETTLE) {
            return false;
        }
        let (Some(m), Some(clear)) = (self.material(Pass::Bake), self.material(Pass::Clear)) else { return false };
        self.lv.owed = false;
        let lamps = self.lv.current.clone();
        let owed_walls = std::mem::take(&mut self.lv.owed_walls);
        let walls = if std::mem::take(&mut self.lv.owed_whole) {
            Some(&[][..])
        } else {
            (!owed_walls.is_empty()).then_some(&owed_walls[..])
        };
        // A heater re-stamped, or a wall went up where no light reaches: the
        // glow is the same, and nothing is redone. A room's fill spreads a
        // light over its whole floor, so a change to any fill redoes
        // everything; a room rebuild that leaves every fill as it was
        // doesn't.
        let fill = fill(w, &lamps[..self.lv.own.min(lamps.len())], self.z);
        let work = if fits && fill == self.lv.fill { redo(&self.lv.baked, &lamps, walls) } else { Redo::Whole };
        if work == Redo::Nothing {
            return false;
        }
        if !fits {
            let rt = render_target(size.0, size.1);
            rt.texture.set_filter(FilterMode::Linear);
            self.lv.fires = Some(rt);
        }
        let (tw, th) = (size.0 as f32, size.1 as f32);
        let t = self.texels as f32;
        set_camera(&Camera2D {
            zoom: vec2(2.0 / tw, 2.0 / th),
            target: vec2(tw / 2.0, th / 2.0),
            render_target: self.lv.fires.clone(),
            ..Default::default()
        });
        m.set_texture("occluders", occ);
        m.set_uniform("map", vec2(w.map.w as f32, w.map.h as f32));
        m.set_uniform("res", vec2(tw, th));
        m.set_uniform("scale", FIRE_SCALE);
        self.lv.draws = 0;
        match work {
            Redo::Nothing => {}
            Redo::Whole => {
                self.lv.fill_meshes = fill_meshes(&w.map, &fill, t);
                self.lv.fill = fill;
                clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
                gl_use_material(&m);
                for mesh in lamp_meshes(&lamps, t).iter().chain(&self.lv.fill_meshes) {
                    draw_mesh(mesh);
                    self.lv.draws += 1;
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
                    for mesh in lamp_meshes(&inside, t).iter().chain(&self.lv.fill_meshes) {
                        draw_mesh(mesh);
                    }
                    self.lv.draws += 2;
                }
                // SAFETY: as above.
                unsafe { get_internal_gl() }.quad_gl.scissor(None);
            }
        }
        gl_use_default_material();
        set_default_camera();
        self.lv.lamps = lamps.len();
        self.lv.baked = lamps;
        self.bakes += 1;
        self.lv.last_bake = now;
        true
    }

    /// Draw the firelight the bake doesn't hold: lights that came since the
    /// last bake, and moving ones. The preset's cap of them cast shadows,
    /// nearest the view first; the rest glow without. Whether it drew.
    fn draw_moving(&mut self, w: &World, centre: Vec2) -> bool {
        // Lights the bake hasn't caught up with; with none owed, it has.
        let baked = &self.lv.baked;
        let waiting: Vec<Lamp> = if self.lv.owed {
            self.lv.current.iter().filter(|l| !baked.contains(l)).copied().collect()
        } else {
            Vec::new()
        };
        let Some(fires) = self.lv.fires.as_ref() else { return false };
        let size = (fires.texture.width(), fires.texture.height());
        if waiting.is_empty() && self.carried.is_empty() && !self.lv.moving_drawn {
            self.moving_lit = (0, 0);
            return false;
        }
        let Some(m) = self.material(Pass::Bake) else { return false };
        let Some(occ) = self.lv.occluders.texture.clone() else { return false };
        if self.lv.moving.as_ref().is_none_or(|t| (t.texture.width(), t.texture.height()) != size) {
            let rt = render_target(size.0 as u32, size.1 as u32);
            rt.texture.set_filter(FilterMode::Linear);
            self.lv.moving = Some(rt);
        }
        // Lights waiting for the bake are still: they cast shadows, as the
        // bake will give them, for the second at most they wait. Moving ones
        // share the preset's cap.
        let cap = self.setting.quality.moving_shadows as usize;
        let moving = cap_shadows(&self.carried, centre, cap);
        let lights: Vec<Lamp> = waiting.iter().chain(&moving).copied().collect();
        let (tw, th) = size;
        set_camera(&Camera2D {
            zoom: vec2(2.0 / tw, 2.0 / th),
            target: vec2(tw / 2.0, th / 2.0),
            render_target: self.lv.moving.clone(),
            ..Default::default()
        });
        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
        m.set_texture("occluders", occ);
        m.set_uniform("map", vec2(w.map.w as f32, w.map.h as f32));
        m.set_uniform("res", vec2(tw, th));
        m.set_uniform("scale", FIRE_SCALE);
        gl_use_material(&m);
        for mesh in lamp_meshes(&lights, self.texels as f32) {
            draw_mesh(&mesh);
        }
        gl_use_default_material();
        set_default_camera();
        self.lv.moving_drawn = !lights.is_empty();
        self.moving_lit = (waiting.len() + moving.iter().filter(|l| l.strength > 0.0).count(), lights.len());
        true
    }

    /// Lights that move, for this frame and until set again: where each is,
    /// in cells, on the level in view, how far it reaches and how bright it
    /// is at the source, in the light field's units. They flicker as fire
    /// does.
    pub fn set_moving(&mut self, lights: impl IntoIterator<Item = (Vec2, f32, f32)>) {
        self.carried = lights
            .into_iter()
            .map(|(p, reach, amount)| Lamp {
                x: p.x,
                y: p.y,
                reach: reach.clamp(0.75, MAX_REACH),
                strength: (amount / 100.0).clamp(0.0, 1.0 / FIRE_SCALE),
                channel: channel_of(Flicker::Fire, p.x.floor() as i32, p.y.floor() as i32),
            })
            .collect();
    }

    /// Whether the lights have changed since the last bake, which waits for
    /// the `SETTLE` to pass.
    pub fn owes_a_bake(&self) -> bool {
        self.lv.owed
    }

    /// The moving lights' firelight at a point, in cells: each channel's
    /// brightness, read back from the GPU. Slow, for tests.
    pub fn moving_at(&self, x: f32, y: f32) -> Option<[f32; 4]> {
        let c = self.texel(&self.lv.moving.as_ref()?.texture.get_texture_data(), x, y)?;
        Some([c.r, c.g, c.b, c.a].map(|v| v / FIRE_SCALE))
    }

    /// Each roofed cell's share of the sky: `[[sky]]`'s `indoor_share`, what
    /// gets through walls and doors, plus the pass its room's windows give
    /// the light field (DESIGN.md §6c). Checked every frame, since the field
    /// sums a room's windows after the map rebuilds it; a room at a time,
    /// so it's cheap. Whether it changed.
    fn update_rooms(&mut self, w: &World) -> bool {
        let (m, z) = (&w.map, self.z);
        // What happens above changes what's open below: the levels above's
        // revisions key the open sky.
        let above: u64 = (z + 1..=0).map(|up| m.level_revision(up)).sum();
        let light = w.defs.lookup("field", "light").map(|f| f as usize);
        let base = w.defs.sky.indoor_share;
        let shares = (1..=m.room_count() as u32)
            .map(|id| {
                // Below the surface no sky gets in, through walls or windows.
                if !m.room_by_id(id).enclosed() || z < 0 {
                    return 0;
                }
                let pass = light.map_or(0.0, |f| w.fields.boundary(f, id).1);
                ((base + pass).clamp(0.0, 1.0) * 255.0).round() as u8
            })
            .collect();
        let key = (m.room_rebuilds, shares, above);
        if self.lv.rooms.is_some() && self.lv.rooms_key.as_ref() == Some(&key) {
            return false;
        }
        let sky = open_sky(m, z);
        self.lv.any_sky = sky.iter().any(|&s| s > 0.0);
        self.lv.sky = sky.clone();
        let mut bytes = vec![0u8; (m.w * m.h * 4) as usize];
        for i in 0..(m.w * m.h) as usize {
            let p = m.pos(i);
            let id = m.room_ids(m.idx(rim_sim::IVec::at(p.x, p.y, z))).0;
            if id > 0 {
                bytes[i * 4] = key.1[id as usize - 1];
            }
            bytes[i * 4 + 1] = (sky[i] * 255.0).round() as u8;
        }
        self.lv.rooms_key = Some(key);
        let img = Image { bytes, width: m.w as u16, height: m.h as u16 };
        match &self.lv.rooms {
            Some(t) if t.width() as i32 == m.w && t.height() as i32 == m.h => t.update(&img),
            _ => {
                let t = Texture2D::from_image(&img);
                t.set_filter(FilterMode::Linear);
                self.lv.rooms = Some(t);
            }
        }
        true
    }

    /// Work out where the sun reaches, if it moved or the occluders changed.
    /// Whether it did.
    fn update_sun(&mut self, w: &World, air: &Air, flash: Flash) -> bool {
        let Some(occ) = self.lv.occluders.texture.clone() else { return false };
        let size = (w.map.w as u32 * self.texels, w.map.h as u32 * self.texels);
        let q = self.setting.quality;
        // A flash is a light of its own for its few frames: one key while it
        // lasts, hard-edged, from where the bolt is.
        // Below the surface no flash reaches either.
        let bolt = flash.strength > FLASH_ON && self.z >= 0;
        // Below the surface the occluders stand as tall as the levels above:
        // the sun reaches only down a shaft, inside its cone.
        let (version, cloud) = (self.lv.occluders.version, air.cloud);
        let key_of = |at, soft| SunKey::new(Some(at), soft, version, q.sun_rebuild, q.sun_steps);
        let (keys, slots) = if !self.lv.any_sky {
            (Vec::new(), Vec::new())
        } else if bolt {
            (vec![key_of((flash.azimuth as f64, FLASH_ELEVATION), 0.0)], Vec::new())
        } else {
            // Cloud softens the shadows; a wider body, more (below).
            let slots = shadow_slots(&self.lit, cloud, q.sky_shadows as usize);
            let soft = if q.soft { penumbra(cloud) } else { 0.0 };
            let at = |id| self.lit.iter().find(|b| b.id == id).map_or((0.0, -1.0), |b| b.at);
            (slots.iter().map(|&id| key_of(at(id), soft)).collect(), slots)
        };
        let key = (bolt, keys, slots);
        let fits =
            self.lv.sunlit.as_ref().is_some_and(|t| (t.texture.width() as u32, t.texture.height() as u32) == size);
        if fits && self.lv.sun_key.as_ref() == Some(&key) {
            return false;
        }
        let Some(m) = self.material(Pass::Sun) else { return false };
        if !fits {
            let rt = render_target(size.0, size.1);
            rt.texture.set_filter(FilterMode::Linear);
            self.lv.sunlit = Some(rt);
        }
        // The keys' quantised bodies, so the result is exactly the key's.
        let wide = |k: usize| key.2.get(k).and_then(|&id| self.lit.iter().find(|b| b.id == id)).map_or(1.0, |b| b.size);
        for (k, name) in ["sun0", "sun1", "sun2", "sun3"].into_iter().enumerate() {
            m.set_uniform(name, key.1.get(k).map_or(Vec4::ZERO, |s| s.toward().extend(s.soft() * wide(k))));
        }
        m.set_uniform("count", key.1.len() as f32);
        (self.lv.slots, self.lv.bolt) = (key.2.clone(), bolt);
        self.lv.sun_key = Some(key);
        let (tw, th) = (size.0 as f32, size.1 as f32);
        m.set_texture("occluders", occ);
        m.set_uniform("map", vec2(w.map.w as f32, w.map.h as f32));
        m.set_uniform("res", vec2(tw, th));
        m.set_uniform("steps", q.sun_steps as f32);
        set_camera(&Camera2D {
            zoom: vec2(2.0 / tw, 2.0 / th),
            target: vec2(tw / 2.0, th / 2.0),
            render_target: self.lv.sunlit.clone(),
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
        let (ambient, direct, _) = self.split(self.sky_color(w, lit), air);
        let sky = ambient + direct.iter().sum::<Vec3>();
        ((sky + bolt) * self.exposure.max(1.0)).max(rgb3(w.defs.sky.rgb_night))
    }

    /// The sky's light `sky` by where it lands. Each shadowed body's
    /// straight light goes through its channel, in its colour; a body
    /// without one lights where the sky does, in its colour; the rest is
    /// the sky's own. While `sunlit` holds a flash's shadows no body has
    /// any: their light is the sky's for those few frames. Below the
    /// surface, with no shaft, straight light lands nowhere. Last, how much
    /// each channel's reach clears the plan's contact shadow: as much as its
    /// light is the day's, so a moon leaves the night's convention be.
    fn split(&self, sky: Vec3, air: &Air) -> (Vec3, [Vec3; 4], [f32; 4]) {
        let (mut ambient, mut direct, mut weights) = (sky, [Vec3::ZERO; 4], [0.0f32; 4]);
        let daylike = (self.sky_light / CONTACT_LIGHT).clamp(0.0, 1.0);
        for b in &self.lit {
            let d = if self.lv.bolt { 0.0 } else { b.direct(air.cloud) };
            ambient -= sky * d;
            match self.lv.slots.iter().position(|&i| i == b.id) {
                Some(k) => (direct[k], weights[k]) = (sky * d * b.rgb, d / DIRECT * daylike),
                None if self.lv.any_sky => ambient += sky * d * b.rgb,
                None => {}
            }
        }
        if self.lv.bolt {
            weights[0] = 1.0;
        }
        let sum: f32 = weights.iter().sum();
        (ambient.max(Vec3::ZERO), direct, if sum > 1.0 { weights.map(|x| x / sum) } else { weights })
    }

    /// A flash's light: how much it brightens the sky everywhere, and the
    /// bolt's own, which lands only where `sunlit` says. A flash struck
    /// since the sun pass last ran lights everything evenly for its frame.
    fn flash_light(&self, flash: Flash) -> (f32, Vec3) {
        if self.lv.bolt {
            (flash.strength * FLASH_AMBIENT, FLASH_RGB * flash.strength * (1.0 - FLASH_AMBIENT))
        } else {
            (flash.strength, Vec3::ZERO)
        }
    }

    /// How each way a roof slopes is lit, as `roof_faces` says, for the sun
    /// now.
    pub fn roof_faces(&self, air: &Air) -> [f32; 5] {
        let direct = |b: &&Body| b.direct(air.cloud);
        let brightest = self.lit.iter().max_by(|a, b| direct(a).total_cmp(&direct(b)));
        roof_faces(brightest.map(|b| b.at), self.lit.iter().map(|b| b.direct(air.cloud)).sum())
    }

    /// The sky's colour and brightness now: white, tinted by `[[sky]]`
    /// tints, times the outdoor light, brightened by a lightning flash.
    fn sky_color(&self, w: &World, flash: f32) -> Vec3 {
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
        let bright = (self.sky_light / 100.0).clamp(0.0, 1.44).sqrt() + flash;
        tinted * bright
    }

    /// Draw the light over the world with multiply blending. Whether it
    /// drew: without the shader the world stays unlit.
    fn draw_multiply(&mut self, w: &World, cam: &Cam, air: &Air, flash: Flash) -> bool {
        let Some(occ) = self.lv.occluders.texture.clone() else { return false };
        // Without the sun pass the sun reaches nowhere; without the bake,
        // no fire does.
        let blank = self.blank.get_or_insert_with(|| Texture2D::from_rgba8(1, 1, &[0, 0, 0, 0])).clone();
        let fires = self.lv.fires.as_ref().map_or(blank.clone(), |t| t.texture.clone());
        let moving = match &self.lv.moving {
            Some(t) if self.lv.moving_drawn => t.texture.clone(),
            _ => blank.clone(),
        };
        let sun = self.lv.sunlit.as_ref().map_or(blank.clone(), |t| t.texture.clone());
        let rooms = self.lv.rooms.clone().unwrap_or(blank);
        let Some(m) = self.material(Pass::Multiply) else { return false };
        // A flash whose shadows are worked out lights the cloud a little and
        // the rest from where it is; one struck since, not yet, lights all.
        let (lit, bolt) = self.flash_light(flash);
        // Below the surface the sky reaches only down a shaft (the rooms'
        // open sky), and night there is darker than night outside.
        let under = self.z < 0;
        let (sky, bolt) = (self.sky_color(w, lit), if under { Vec3::ZERO } else { bolt });
        let (ambient, direct, weights) = self.split(sky, air);
        let def = &w.defs.sky;
        let (mw, mh) = (w.map.w as f32, w.map.h as f32);
        m.set_texture("occluders", occ);
        m.set_texture("sunlit", sun);
        m.set_texture("rooms", rooms);
        m.set_texture("moving", moving);
        // The eye adapts to the sky the view holds: on the surface all of
        // it, below it what comes down the shafts in view.
        let seen = if under { self.view_sky(w, cam) } else { 1.0 };
        let target = exposure_for(sky * seen);
        // About a second to settle, by the clock, and no more than a 30th of
        // a second's worth in one frame, so a stalled frame doesn't jump.
        self.exposure = if self.exposure > 0.0 {
            self.exposure + (target - self.exposure) * (1.0 - (-1.6 * get_frame_time().min(1.0 / 30.0)).exp())
        } else {
            target
        };
        m.set_uniform("exposure", self.exposure);
        m.set_uniform("ambient", ambient);
        for (k, name) in ["direct0", "direct1", "direct2", "direct3"].into_iter().enumerate() {
            m.set_uniform(name, direct[k]);
        }
        m.set_uniform("weights", Vec4::from_array(weights));
        m.set_uniform("bolt", bolt);
        m.set_uniform("night", rgb3(def.rgb_night) * if under { UNDERGROUND } else { 1.0 });
        let fire = rgb3(def.rgb_fire) * 1.2;
        for (k, name) in ["ch0", "ch1", "ch2", "ch3"].into_iter().enumerate() {
            m.set_uniform(name, channel_colour(fire, k, get_time()));
        }
        m.set_uniform("cell", vec2(1.0 / mw, 1.0 / mh));
        m.set_uniform("lres", vec2(mw, mh) * self.texels as f32);
        // With no sun path the contact shadow fades with daylight instead.
        let day = if self.lit.is_empty() { (air.light / 100.0).clamp(0.0, 1.0) } else { 0.0 };
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

    /// The lighting setting that runs: under `auto`, the preset it's at.
    pub fn setting(&self) -> &Setting {
        &self.setting
    }

    /// The player chose a setting: under `auto`, it watches afresh, and
    /// frames timed before are let go.
    pub fn set(&mut self, setting: Setting) {
        self.setting = setting;
        self.watch = Watch::default();
        self.chosen = self.chosen.wrapping_add(1);
    }

    /// Whether `auto` can time the lighting here: None until GL has said.
    pub fn can_time_cost(&self) -> Option<bool> {
        match self.cost {
            Timer::Untried => None,
            Timer::Unavailable => Some(false),
            Timer::Ready(_) => Some(true),
        }
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

    /// The timer `auto` reads, while it has a preset to step down to. Not
    /// while the bench times each pass: queries don't nest.
    fn cost(&mut self) -> Option<&mut CostTimer> {
        if !self.setting.auto || self.setting.preset == 0 || self.gpu_timing {
            return None;
        }
        // GL names itself once it has a context; until then, ask again later.
        if matches!(self.cost, Timer::Untried) && !gl_renderer().is_empty() {
            self.cost = CostTimer::new().map_or_else(
                || {
                    // Apple's GPU draws a frame in tiles, all passes at once,
                    // so there is nothing to read the lighting's share from.
                    eprintln!(
                        "rim: lighting auto: {} can't time the lighting on its own, so it stays at {}",
                        gl_renderer(),
                        self.setting.name()
                    );
                    Timer::Unavailable
                },
                Timer::Ready,
            );
        }
        match &mut self.cost {
            Timer::Ready(c) => Some(c),
            _ => None,
        }
    }

    /// Under `auto`, a new frame: judge the one timed `COST_FRAMES` ago.
    fn cost_frame(&mut self) {
        let tag = (self.setting.preset, self.chosen);
        let Some(timed) = self.cost().map(|c| c.frame(tag)) else { return };
        // A frame from before the player last chose is no longer theirs.
        let Some(((preset, _), us)) = timed.filter(|((_, c), _)| *c == tag.1) else { return };
        self.cost_frames += 1;
        if let Some(mean) = self.setting.watch(&mut self.watch, get_time(), preset, us) {
            eprintln!(
                "rim: lighting auto: {:.1} ms a frame on the GPU, over {:.1}; down to {}",
                mean / 1e3,
                crate::quality::AUTO_BUDGET_US / 1e3,
                self.setting.name()
            );
        }
    }

    fn cost_begin(&mut self, k: usize) {
        if let Some(c) = self.cost() {
            c.begin(k);
        }
    }

    /// Close the query `cost_begin` opened, whatever changed since.
    fn cost_end(&mut self) {
        if let Timer::Ready(c) = &mut self.cost {
            c.end();
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

    /// Core's sky at `hours` past the start (06:00 on the first day, whose
    /// night has a full moon): which bodies cast shadows, with `cap` slots.
    fn shadows_at(hours: f64, cap: usize) -> Vec<String> {
        let mods = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
        let mut sim = rim_sim::Sim::build(&mods, 1, &|id| id == "core", 48).unwrap();
        sim.world.tick = (hours / 24.0 * rim_sim::TICKS_PER_DAY as f64) as u64;
        for _ in 0..40 {
            sim.step();
        }
        let w = &sim.world;
        let slots = shadow_slots(&Light::default().bodies(w, crate::sky::Air::read(w).light).0, 0.0, cap);
        slots.iter().map(|&i| w.defs.sky_bodies[i].id.clone()).collect()
    }

    #[test]
    fn with_one_shadow_slot_the_sun_casts_by_day_and_the_moon_by_night() {
        assert_eq!(shadows_at(6.0, 1), ["core:sun"], "noon");
        assert_eq!(shadows_at(18.0, 1), ["core:moon"], "midnight, under a full moon");
        assert_eq!(shadows_at(18.0, 4), ["core:moon"], "the sun is down, however many slots");
        // Half its cycle on, the moon is new at noon: the sun's.
        assert_eq!(shadows_at(6.0 + 8.0 * 24.0, 4), ["core:sun"], "a new moon beside the sun casts nothing");
    }

    #[test]
    fn a_moon_lights_in_its_colour_but_leaves_the_nights_contact_shadow() {
        let moon = Body { id: 1, at: (90.0, 40.0), share: 1.0, rgb: vec3(0.5, 1.0, 0.6), size: 3.0, shadows: true };
        // Moonlight is the picture's alone: the sim's light is 0, the sky's 1.5.
        let mut light = Light { lit: vec![moon], sky_light: 1.5, ..Default::default() };
        (light.lv.slots, light.lv.any_sky) = (vec![1], true);
        let night = crate::sky::Air::default();
        let (ambient, direct, weights) = light.split(Vec3::ONE, &night);
        assert!((direct[0] - vec3(0.5, 1.0, 0.6) * DIRECT).length() < 1e-5, "its straight light, green: {direct:?}");
        assert!((ambient - Vec3::splat(1.0 - DIRECT)).length() < 1e-5, "the rest is the sky's: {ambient:?}");
        assert!(weights[0] < 0.2, "moonlight clears little of the contact shadow: {weights:?}");
        light.sky_light = 90.0;
        assert!((light.split(Vec3::ONE, &night).2[0] - 1.0).abs() < 1e-5, "daylight clears it");
        light.sky_light = 1.5;
        // Without a slot its light lands where the sky's does, still green.
        light.lv.slots.clear();
        let (ambient, direct, _) = light.split(Vec3::ONE, &night);
        assert_eq!(direct, [Vec3::ZERO; 4]);
        assert!((ambient - (Vec3::splat(1.0 - DIRECT) + vec3(0.5, 1.0, 0.6) * DIRECT)).length() < 1e-5);
        // Below the surface with no shaft, straight light lands nowhere.
        light.lv.any_sky = false;
        assert!((light.split(Vec3::ONE, &night).0 - Vec3::splat(1.0 - DIRECT)).length() < 1e-5);
    }

    #[test]
    fn the_brightest_straight_light_gets_the_slots_and_only_bodies_that_may() {
        let body =
            |id, elev: f64, share, shadows| Body { id, at: (90.0, elev), share, rgb: Vec3::ONE, size: 1.0, shadows };
        let sky =
            [body(0, 50.0, 0.2, true), body(1, 50.0, 0.7, true), body(2, 50.0, 0.9, false), body(3, -5.0, 1.0, true)];
        assert_eq!(shadow_slots(&sky, 0.0, 4), [1, 0], "brightest first; none that may not, nor below the horizon");
        assert_eq!(shadow_slots(&sky, 0.0, 1), [1]);
        assert!(shadow_slots(&sky, 0.0, 0).is_empty());
        let low = [body(0, 2.0, 0.9, true), body(1, 60.0, 0.5, true)];
        assert_eq!(shadow_slots(&low, 0.0, 1), [1], "a body near the horizon gives less straight light");
    }

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
    fn the_moving_lights_nearest_the_view_cast_the_shadows() {
        let at = |x: f32| Lamp { x, y: 0.5, reach: 4.0, strength: 0.6, channel: 0 };
        let lights: Vec<Lamp> = [9.5, 1.5, 5.5, 3.5, 7.5].into_iter().map(at).collect();
        let capped = cap_shadows(&lights, vec2(0.0, 0.5), 2);
        let order: Vec<(f32, bool)> = capped.iter().map(|l| (l.x, l.strength > 0.0)).collect();
        assert_eq!(order, [(1.5, true), (3.5, true), (5.5, false), (7.5, false), (9.5, false)]);
        assert!(capped.iter().all(|l| l.strength.abs() == 0.6), "past the cap they glow as bright");
        assert!(cap_shadows(&lights, vec2(0.0, 0.5), 8).iter().all(|l| l.strength > 0.0), "under the cap, all");
    }

    #[test]
    fn a_wide_opening_passes_on_half_a_light_at_most_and_a_wall_stops_it() {
        let mods = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
        let mut s = rim_sim::Sim::new(&mods, 3).expect("mods load");
        let w = &mut s.world;
        let (air, grass) = (
            w.defs.terrain.iter().position(|d| d.air).unwrap() as rim_sim::defs::DefId,
            w.defs.lookup("terrain", "grass").unwrap(),
        );
        // Open grass up top, a 3×3 pit two cells from a campfire.
        let o = w.colony_center().unwrap().offset(12, 12);
        for y in -2..6 {
            for x in -4..6 {
                let p = o.offset(x, y);
                for e in [w.map.fixture_at(p), w.map.item_at(p), w.map.floor_at(p)].into_iter().flatten() {
                    w.despawn_thing(e);
                }
                w.map.set_terrain(p, grass, 100);
            }
        }
        for p in (0..3).flat_map(|y| (0..3).map(move |x| o.offset(x, y))) {
            w.map.set_terrain(p, air, 0);
        }
        let campfire = w.defs.thing_id("campfire").unwrap();
        w.spawn_fixture(campfire, o.offset(-2, 1), false).unwrap();
        let fire = lamps(w, 0)[0];
        let across = lamps_across(w, -1);
        assert_eq!(across.len(), 9, "a light at each cell of the pit");
        let total: f32 = across.iter().map(|l| l.strength).sum();
        assert!(
            total <= fire.strength * ACROSS + 1e-4,
            "{total} from {}: no brighter below than half the fire",
            fire.strength
        );
        // A wall between the fire and the pit: nothing gets through.
        let wall = w.defs.thing_id("wall").unwrap();
        for y in -1..4 {
            w.spawn_fixture(wall, o.offset(-1, y), false).unwrap();
        }
        assert!(lamps_across(w, -1).is_empty(), "the wall stands between");
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
