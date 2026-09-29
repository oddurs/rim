//! One frame drawn (DESIGN.md §6): the ground, the chunk meshes and the
//! things on them, pawns, weather, light and the UI, into the world target
//! and out to the screen, with each pass timed for the bench.

use super::*;

/// CPU time of each render pass last frame, in µs. This is building the
/// batches; the GL work happens when the frame ends.
#[derive(Clone, Copy, Default, Debug)]
pub struct RenderTimes {
    pub ground: f64,
    /// Painting, excluding `gl`.
    pub things: f64,
    /// Handing work to GL mid-frame: the chunk meshes, the batch before
    /// each layer, the pawns' figures, and a scaled world's target to the
    /// screen. Submission,
    /// like macroquad's end of frame: a software rasteriser does its
    /// drawing here, a GPU driver only queues.
    pub gl: f64,
    pub pawns: f64,
    pub weather: f64,
    pub light: f64,
    pub ui: f64,
}

impl RenderTimes {
    pub fn rows(&self) -> [(&'static str, f64); 7] {
        [
            ("ground", self.ground),
            ("things", self.things),
            ("gl", self.gl),
            ("pawns", self.pawns),
            ("weather", self.weather),
            ("light", self.light),
            ("ui", self.ui),
        ]
    }

    /// The world's CPU: everything but the UI, which has its own budget
    /// (DESIGN.md §11), and GL submission, which is the driver's.
    pub fn world(&self) -> f64 {
        self.ground + self.things + self.pawns + self.weather + self.light
    }
}

/// Make `world_target` match the screen at the render scale. The world
/// draws there even at full scale: it takes several passes (the batch, the
/// chunk meshes' layers), and on macOS's GL each pass on the window's own
/// framebuffer cost the frame far more than one into a texture. Measured
/// at 2x: 13-33 ms a frame to submit straight to the screen, 2-8 ms into
/// a target and one copy out.
fn update_world_target(app: &mut App) {
    let dpi = screen_dpi_scale();
    let scale = app.render_scale.unwrap_or(1.0);
    let (w, h) = ((screen_width() * dpi * scale).round(), (screen_height() * dpi * scale).round());
    if w < 1.0 || h < 1.0 {
        app.world_target = None;
        return;
    }
    let fits = app.world_target.as_ref().is_some_and(|t| t.texture.width() == w && t.texture.height() == h);
    if !fits {
        let t = render_target(w as u32, h as u32);
        // Pixel for pixel at full scale; smoothed when stretched.
        t.texture.set_filter(if scale < 1.0 { FilterMode::Linear } else { FilterMode::Nearest });
        app.world_target = Some(t);
    }
}

/// Copy a texture as it is: no blending, alpha forced to 1. Translucent
/// things drawn into the world's target (fog, plans, rain) leave its alpha
/// below 1, and blending that over the cleared screen would darken them.
fn blit_material() -> Option<Material> {
    const VERTEX: &str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
varying lowp vec2 uv;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    uv = texcoord;
}";
    const FRAGMENT: &str = "#version 100
varying lowp vec2 uv;
uniform sampler2D Texture;
void main() {
    gl_FragColor = vec4(texture2D(Texture, uv).rgb, 1.0);
}";
    let m = load_material(
        ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
        MaterialParams {
            pipeline_params: PipelineParams { color_blend: None, ..Default::default() },
            ..Default::default()
        },
    );
    // Without it the world still shows, darkened under translucency; say so.
    m.map_err(|e| eprintln!("render scale blit shader failed, blending instead: {e}")).ok()
}

/// How long changing level takes to fade from one level's frame to the
/// next's, in seconds: twelve frames at 60 Hz, so no frame moves the
/// brightness a tenth of the way from a noon surface to a cellar.
const LEVEL_FADE: f32 = 0.2;

/// Draw a frame at an alpha, its own alpha ignored: what the world left in
/// it is not transparency, as for `blit_material`.
fn fade_material() -> Option<Material> {
    const VERTEX: &str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
varying lowp vec2 uv;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    uv = texcoord;
}";
    const FRAGMENT: &str = "#version 100
varying lowp vec2 uv;
uniform sampler2D Texture;
uniform lowp float alpha;
void main() {
    gl_FragColor = vec4(texture2D(Texture, uv).rgb, alpha);
}";
    use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
    let over = BlendState::new(
        Equation::Add,
        BlendFactor::Value(BlendValue::SourceAlpha),
        BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
    );
    let m = load_material(
        ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
        MaterialParams {
            uniforms: vec![UniformDesc::new("alpha", UniformType::Float1)],
            pipeline_params: PipelineParams { color_blend: Some(over), ..Default::default() },
            ..Default::default()
        },
    );
    // Without it a level change cuts instead of fading; say so.
    m.map_err(|e| eprintln!("level fade shader failed, cutting instead: {e}")).ok()
}

pub fn render(app: &mut App) {
    let mut clock = std::time::Instant::now();
    let mut lap = || {
        let us = clock.elapsed().as_secs_f64() * 1e6;
        clock = std::time::Instant::now();
        us
    };
    let mut t = RenderTimes::default();
    // Light's own targets first, while the camera is the screen's.
    let air = sky::Air::read(&app.sim.world);
    // The world's pixels a cell: the zoom, at the screen's density and the
    // render scale it's drawn at.
    let px_per_cell = app.cam.zoom * screen_dpi_scale() * app.render_scale.unwrap_or(1.0);
    // Roofs first: the light reads their heights. Only a room rebuild
    // works them out again.
    app.roofs.update(&app.sim.world);
    // The sky first: a flash that strikes this frame is lit from its bolt
    // this frame.
    app.sky.update(&air, app.now);
    let centre = vec2(app.cam.x, app.cam.y);
    app.light.clock(app.now, app.dt);
    app.light.prepare(&app.sim.world, &air, px_per_cell, &app.roofs.height, app.sky.flash(), centre, app.cam.z);
    t.light = lap();
    // Changing level: the frame the level left drew stays, and fades out
    // over the new one, so the light doesn't jump (DESIGN.md §6e, Depth).
    if app.cam.z != app.drawn_z {
        app.drawn_z = app.cam.z;
        let kept = app.world_target.take();
        // Changing again mid-fade: what the screen shows is the mix, so
        // that is what fades out now, not the last level's frame alone.
        if let (Some(rt), Some((from, k)), Some(m)) = (&kept, &app.fade, &app.fade_blit) {
            let (w, h) = (rt.texture.width(), rt.texture.height());
            set_camera(&Camera2D {
                zoom: vec2(2.0 / w, 2.0 / h),
                target: vec2(w / 2.0, h / 2.0),
                render_target: Some(rt.clone()),
                ..Default::default()
            });
            m.set_uniform("alpha", (1.0 - *k).max(0.0));
            gl_use_material(m);
            draw_texture_ex(
                &from.texture,
                0.0,
                0.0,
                WHITE,
                DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() },
            );
            gl_use_default_material();
            set_default_camera();
        }
        app.fade = kept.map(|rt| (rt, 0.0));
        // The eye takes the new level's light at once, and the fade carries
        // the change: a blend of two settled frames, the same share a frame
        // however fast the frames come.
        app.light.adapt_now();
    }
    update_world_target(app);
    let (sw, sh) = (screen_width(), screen_height());
    if let Some(rt) = &app.world_target {
        // The same screen points, into the target's pixels.
        set_camera(&Camera2D {
            zoom: vec2(2.0 / sw, 2.0 / sh),
            target: vec2(sw / 2.0, sh / 2.0),
            render_target: Some(rt.clone()),
            ..Default::default()
        });
    }
    app.ground.update(&app.sim.world, app.cam.z);
    app.water.update(&app.sim.world, app.cam.z, app.now);
    t.ground = lap();
    app.worksites.follow_level(app.cam.z);
    app.worksites.update(&app.sim.world, app.cam.zoom >= worksite::DETAIL_ZOOM);
    let counts = draw::things(app);
    t.gl = app.meshes.submit_us;
    t.things = lap() - t.gl;
    draw::pawns(app);
    t.gl += app.figures.gl_us;
    t.pawns = lap() - app.figures.gl_us;
    // Underground no weather falls, and no roof shows (DESIGN.md §6d).
    if app.cam.z >= 0 {
        app.sky.weather(&app.sim.world, &app.cam, &air);
    }
    t.weather = lap();
    // Every level is lit by its own light (DESIGN.md §6e, Depth).
    app.light.multiply(&app.sim.world, &app.cam, &air, app.sky.flash());
    // Roofs are outdoors whatever is under them: after the light, lit by
    // the sky and the sun. The house under the pointer lifts its roof.
    let alpha = roof::Roofs::alpha(app.cam.zoom);
    if alpha > 0.0 && app.cam.z == 0 {
        let lifted = app.hover_cell.map_or(0, |p| app.roofs.house_at(&app.sim.world, p));
        let tint = app.light.outdoor(&app.sim.world, &air, app.sky.flash());
        let faces = app.light.roof_faces(&air);
        app.roofs.draw(&app.sim.world, &app.cam, draw::visible(app), alpha, lifted, tint, faces);
    }
    t.light += lap() - app.light.shade_gl_us;
    t.gl += app.light.shade_gl_us;
    if let Some(rt) = &app.world_target {
        set_default_camera();
        if app.blit.is_none() {
            app.blit = blit_material();
        }
        if let Some(m) = &app.blit {
            gl_use_material(m);
        }
        let size = DrawTextureParams { dest_size: Some(vec2(sw, sh)), ..Default::default() };
        draw_texture_ex(&rt.texture, 0.0, 0.0, WHITE, size.clone());
        gl_use_default_material();
        if let Some((from, k)) = &mut app.fade {
            // At most a 60th of a second a frame: never over in fewer than
            // twelve frames, however slow they come.
            *k += app.dt.min(1.0 / 60.0) / LEVEL_FADE;
            if app.fade_blit.is_none() {
                app.fade_blit = fade_material();
            }
            if let (Some(m), true) = (&app.fade_blit, *k < 1.0) {
                m.set_uniform("alpha", 1.0 - *k);
                gl_use_material(m);
                draw_texture_ex(&from.texture, 0.0, 0.0, WHITE, size);
                gl_use_default_material();
            }
            if *k >= 1.0 || app.fade_blit.is_none() {
                app.fade = None;
            }
        }
        // Switching cameras hands the target's batch to GL: submission,
        // like the meshes'.
        t.gl += lap();
    }
    app.marks.update(&app.sim.world);
    draw::world_ui(app);
    let scene = overlay::scene(app);
    overlay::draw(&scene, &app.palette, app.cam.zoom);
    let readouts = draw::readouts(app);
    // Stack counts, in the UI's text: shaped into the same atlas, drawn in
    // the same batch as the UI. After lighting, so they read at night.
    let dpi = screen_dpi_scale();
    let mut labels = Vec::with_capacity(counts.len() * 2 + readouts.len() * 2);
    for (x, y, text) in readouts {
        let look = ([0.0, 0.0, 0.0, 0.7], [0.91, 0.93, 0.9, 1.0]);
        labels.extend(overlay::shadowed(&mut app.ui.text, &text, 12.0, 600, (x, y), look, dpi));
    }
    for (x, y, n) in counts {
        let look = ([0.0, 0.0, 0.0, 0.6], [1.0, 1.0, 1.0, 1.0]);
        labels.extend(overlay::shadowed(&mut app.ui.text, &n.to_string(), 13.0, 600, (x, y), look, dpi));
    }
    labels.extend(overlay::chips(&scene, &app.palette, &mut app.ui.text, dpi));
    upload_atlas(&mut app.ui, &app.atlas);
    let white = app.ui.text.atlas.white_texel();
    draw::ui(&labels, &app.atlas, white, dpi);
    // The UI was laid out before this frame's pan, zoom and sim step:
    // names and bubbles follow their pawns to where the world just drew them.
    let cam = (app.cam.x, app.cam.y, app.cam.zoom * dpi);
    let screen = (screen_width() * dpi, screen_height() * dpi);
    let frac = app.tick_frac();
    let came = app.motion.came_from();
    rim_ui::reanchor(&mut app.last_draw, &mut app.last_anchored, &app.sim.world, cam, screen, frac, &came);
    draw::ui(&app.last_draw, &app.atlas, white, dpi);
    t.ui = lap();
    app.render_us = t;
    app.frames.record(app.meshes.calls + app.figures.calls + app.light.shade_calls, &app.render_us.rows());
}

/// Glyphs shaped since the last frame go to the GPU.
pub fn upload_atlas(ui: &mut Ui, atlas: &Texture2D) {
    if ui.text.atlas.dirty {
        let a = &ui.text.atlas;
        atlas.update(&Image { bytes: a.pixels.clone(), width: a.size as u16, height: a.size as u16 });
        ui.text.atlas.dirty = false;
    }
}
