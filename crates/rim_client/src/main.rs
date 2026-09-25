//! rim client: renders the simulation and turns input into Commands.
//! The client never mutates the world directly.
//!
//! Each frame: raw input goes to the UI first (rim_ui routes it against the
//! last layout; panels swallow clicks). UI actions apply, then whatever the
//! UI didn't take drives the world. The HUD itself is core's UI mod.

mod atlas;
mod autotest;
mod bench;
mod cli;
mod draw;
mod mesh;
mod sky;

use macroquad::prelude::*;
use rim_sim::defs::{DefId, Targets};
use rim_sim::hecs::Entity;
use rim_sim::order;
use rim_sim::world::{Faction, Pawn};
use rim_sim::{Command, IVec, Sim};
use rim_ui::view::{ClientView, ToolView, UiAction};
use rim_ui::Ui;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    Select,
    Designate(DefId),
    Build(DefId),
    Cancel,
}

/// A toolbar entry: generated from defs, addressed by a stable key so UI
/// scripts can name it ("designate:core:chop", "build:core:wall").
pub struct ToolDef {
    pub key: String,
    pub label: String,
    pub tool: Tool,
    pub color: Color,
}

/// The lowest zoom, in points per tile: a 250-cell map fits a 1080p screen.
pub const MIN_ZOOM: f32 = 4.0;

pub struct Cam {
    /// Center of the view, in tiles.
    pub x: f32,
    pub y: f32,
    /// Logical points per tile.
    pub zoom: f32,
}

impl Cam {
    pub fn to_screen(&self, wx: f32, wy: f32) -> (f32, f32) {
        ((wx - self.x) * self.zoom + screen_width() / 2.0, (wy - self.y) * self.zoom + screen_height() / 2.0)
    }
    pub fn to_world(&self, sx: f32, sy: f32) -> (f32, f32) {
        ((sx - screen_width() / 2.0) / self.zoom + self.x, (sy - screen_height() / 2.0) / self.zoom + self.y)
    }
    pub fn tile_at(&self, sx: f32, sy: f32) -> IVec {
        let (wx, wy) = self.to_world(sx, sy);
        IVec::new(wx.floor() as i32, wy.floor() as i32)
    }
}

pub struct App {
    pub sim: Sim,
    pub ui: Ui,
    pub atlas: Texture2D,
    pub cam: Cam,
    pub tool: Tool,
    pub tools: Vec<ToolDef>,
    /// The material last picked for each buildable, so nobody picks wood
    /// forty times. Falls back to whatever the colony has most of.
    pub stuff_for: Vec<(DefId, DefId)>,
    pub selected: Option<Entity>,
    pub drag_start: Option<IVec>,
    pub paused: bool,
    pub speed: u32,
    pub show_profiler: bool,
    pub show_devtools: bool,
    /// Field layer drawn over the map, if any (cycled with O).
    pub overlay: Option<usize>,
    /// What a right-click would do, recomputed only when the cursor moves to
    /// another tile or the selection changes.
    pub hint: Option<String>,
    hint_key: Option<(Entity, IVec, Option<Entity>)>,
    /// Where the last order landed, and when, for a brief marker.
    pub order_flash: Option<(IVec, f64)>,
    /// The pointer was over UI last frame (world hover is suppressed).
    pub mouse_over_ui: bool,
    /// The UI's draw list from the last frame.
    pub last_draw: Vec<rim_ui::paint::Draw>,
    /// Profiler rows, refreshed a few times a second.
    profile: (Vec<(String, f64)>, Vec<String>, f64),
    acc: f64,
    pan_anchor: Option<(f32, f32)>,
    /// Lighting and weather on screen.
    pub sky: sky::Sky,
    /// The terrain, baked into a texture.
    pub ground: draw::Ground,
    /// What each render pass cost last frame.
    pub render_us: RenderTimes,
    /// Floors, items and fixtures, cached per chunk on the GPU.
    pub meshes: mesh::Meshes,
    /// Every mod's sprites, packed at load.
    pub world_atlas: atlas::WorldAtlas,
    /// Input subscriber for wheel events (see `Wheel`).
    wheel_sub: usize,
}

/// Seconds since the last frame, clamped: macroquad's value is raw, so the
/// first frame (load time) or a window drag would otherwise jump the camera
/// and run a burst of sim ticks.
fn frame_time() -> f32 {
    get_frame_time().min(0.1)
}

/// Every key a binding can name, with its name. Letters and digits are
/// themselves; the rest are spelled out.
const KEY_NAMES: &[(KeyCode, &str)] = &[
    (KeyCode::A, "a"),
    (KeyCode::B, "b"),
    (KeyCode::C, "c"),
    (KeyCode::D, "d"),
    (KeyCode::E, "e"),
    (KeyCode::F, "f"),
    (KeyCode::G, "g"),
    (KeyCode::H, "h"),
    (KeyCode::I, "i"),
    (KeyCode::J, "j"),
    (KeyCode::K, "k"),
    (KeyCode::L, "l"),
    (KeyCode::M, "m"),
    (KeyCode::N, "n"),
    (KeyCode::O, "o"),
    (KeyCode::P, "p"),
    (KeyCode::Q, "q"),
    (KeyCode::R, "r"),
    (KeyCode::S, "s"),
    (KeyCode::T, "t"),
    (KeyCode::U, "u"),
    (KeyCode::V, "v"),
    (KeyCode::W, "w"),
    (KeyCode::X, "x"),
    (KeyCode::Y, "y"),
    (KeyCode::Z, "z"),
    (KeyCode::Key0, "0"),
    (KeyCode::Key1, "1"),
    (KeyCode::Key2, "2"),
    (KeyCode::Key3, "3"),
    (KeyCode::Key4, "4"),
    (KeyCode::Key5, "5"),
    (KeyCode::Key6, "6"),
    (KeyCode::Key7, "7"),
    (KeyCode::Key8, "8"),
    (KeyCode::Key9, "9"),
    (KeyCode::F1, "f1"),
    (KeyCode::F2, "f2"),
    (KeyCode::F3, "f3"),
    (KeyCode::F4, "f4"),
    (KeyCode::F5, "f5"),
    (KeyCode::F6, "f6"),
    (KeyCode::F7, "f7"),
    (KeyCode::F8, "f8"),
    (KeyCode::F9, "f9"),
    (KeyCode::F10, "f10"),
    (KeyCode::F11, "f11"),
    (KeyCode::F12, "f12"),
    (KeyCode::Space, "space"),
    (KeyCode::Escape, "escape"),
    (KeyCode::Tab, "tab"),
    (KeyCode::Enter, "enter"),
    (KeyCode::Backspace, "backspace"),
    (KeyCode::Delete, "delete"),
    (KeyCode::Left, "left"),
    (KeyCode::Right, "right"),
    (KeyCode::Up, "up"),
    (KeyCode::Down, "down"),
    (KeyCode::Home, "home"),
    (KeyCode::End, "end"),
    (KeyCode::PageUp, "pageup"),
    (KeyCode::PageDown, "pagedown"),
    (KeyCode::Minus, "-"),
    (KeyCode::Equal, "="),
    (KeyCode::Comma, ","),
    (KeyCode::Period, "."),
    (KeyCode::Slash, "/"),
    (KeyCode::Semicolon, ";"),
    (KeyCode::Apostrophe, "'"),
    (KeyCode::LeftBracket, "["),
    (KeyCode::RightBracket, "]"),
    (KeyCode::Backslash, "\\"),
    (KeyCode::GraveAccent, "`"),
];

/// A character from the text path that a text input should keep. Control
/// characters are the keys the input already handles; the private-use
/// range is how macOS spells its arrow, home, end and function keys in
/// the same stream.
fn typed_char(c: char) -> Option<char> {
    let private = ('\u{e000}'..='\u{f8ff}').contains(&c);
    (!c.is_control() && !private).then_some(c)
}

#[cfg(test)]
mod tests {
    #[test]
    fn function_keys_are_not_text() {
        assert_eq!(super::typed_char('a'), Some('a'));
        assert_eq!(super::typed_char('é'), Some('é'));
        assert_eq!(super::typed_char(' '), Some(' '));
        assert_eq!(super::typed_char('\u{f701}'), None, "macOS down arrow");
        assert_eq!(super::typed_char('\u{8}'), None, "backspace");
        assert_eq!(super::typed_char('\u{1b}'), None, "escape");
    }
}

/// The name a binding uses for a key, if it has one.
pub fn key_name(code: KeyCode) -> Option<&'static str> {
    KEY_NAMES.iter().find(|(c, _)| *c == code).map(|(_, n)| *n)
}

/// Where a per-player file is kept: the platform's data directory, so it
/// follows the player, not the save.
fn player_file(name: &str) -> Option<PathBuf> {
    layout_path().map(|p| p.with_file_name(name))
}

/// Where the UI layout is kept: the platform's per-user data directory,
/// so it follows the player, not the save.
fn layout_path() -> Option<PathBuf> {
    let var = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let base = if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Library/Application Support"))
    } else if cfg!(windows) {
        var("APPDATA")
    } else {
        var("XDG_DATA_HOME").or_else(|| var("HOME").map(|h| h.join(".local/share")))
    };
    base.map(|b| b.join("rim").join("ui-layout.toml"))
}

/// Window and renderer settings. Reasoning: docs/engineering/dependencies.md.
fn conf() -> macroquad::conf::Conf {
    use macroquad::miniquad::conf::{AppleGfxApi, LinuxBackend, Platform};
    macroquad::conf::Conf {
        miniquad_conf: Conf {
            window_title: "rim".to_owned(),
            window_width: 1600,
            window_height: 960,
            window_resizable: true,
            high_dpi: true,
            // No MSAA: shapes are drawn at the display's resolution and the
            // UI is anti-aliased in its glyph atlas; 4x costs real GPU
            // bandwidth at Retina sizes (and software GL in CI).
            sample_count: 1,
            platform: Platform {
                // Vsync where the platform honours it (macOS paces frames
                // with the display itself and ignores this).
                swap_interval: Some(1),
                // The sim runs every frame, so never block waiting for input.
                blocking_event_loop: false,
                // Metal can't run the GLSL lighting shader (sky.rs).
                apple_gfx_api: AppleGfxApi::OpenGl,
                // Wayland support is marked unstable upstream: X11 first.
                linux_backend: LinuxBackend::X11WithWaylandFallback,
                linux_wm_class: "rim",
                ..Default::default()
            },
            ..Default::default()
        },
        // Batches: a draw call ends when it runs out of room. Indices are the
        // limit (6 per rectangle, 60 per circle), so give them 1.5x the
        // vertices. Vertex capacity must stay under 65,536 (u16 indices).
        draw_call_vertex_capacity: 16_000,
        draw_call_index_capacity: 24_000,
        ..Default::default()
    }
}

async fn fail(e: String) {
    loop {
        clear_background(Color::from_rgba(30, 20, 20, 255));
        draw_text("rim failed to start", 40.0, 60.0, 36.0, WHITE);
        for (i, line) in e.lines().enumerate() {
            draw_text(line, 40.0, 110.0 + i as f32 * 24.0, 22.0, Color::from_rgba(255, 180, 160, 255));
        }
        next_frame().await
    }
}

fn main() {
    // Subcommands that need no window run before one opens, so they work
    // on headless CI machines.
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("test") => std::process::exit(cli::test(&args[2..])),
        Some("check") => std::process::exit(cli::check(&args[2..])),
        _ => {}
    }
    macroquad::Window::from_config(conf(), game());
}

async fn game() {
    // Find the system fonts on another thread while mods load and the map
    // generates (from the disk cache: a few ms; a first run scans them all).
    rim_ui::fontcache::preload();
    let args: Vec<String> = std::env::args().collect();
    let seed = args.windows(2).find(|w| w[0] == "--seed").and_then(|w| w[1].parse().ok()).unwrap_or_else(|| {
        // The autotest is a test: the same map every run unless asked
        // (scripts/autotest-sweep.sh covers other maps). Play gets a new one.
        if args.iter().any(|a| a == "--autotest") {
            7
        } else {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        }
    });
    let ui_scale: f32 = args.windows(2).find(|w| w[0] == "--ui-scale").and_then(|w| w[1].parse().ok()).unwrap_or(1.0);

    let bench = args.iter().any(|a| a == "--bench-render");
    let sim = match find_mods().ok_or_else(|| "could not find a mods/ directory".to_string()).and_then(|d| {
        if bench {
            let n = args.windows(2).find(|w| w[0] == "--sprite-mods").and_then(|w| w[1].parse().ok()).unwrap_or(0);
            bench::world(&d, seed, n)
        } else {
            Sim::new(&d, seed)
        }
    }) {
        Ok(s) => s,
        Err(e) => return fail(e).await,
    };
    let mut ui = match Ui::new(rim_ui::mods_of(&sim), screen_dpi_scale(), ui_scale) {
        Ok(u) => u,
        Err(e) => return fail(format!("UI failed to start: {e}")).await,
    };
    // Window positions are the player's, per machine: not in a save game,
    // and not touched by the autotest.
    let layout_file = if args.iter().any(|a| a == "--autotest") { None } else { layout_path() };
    if let Some(text) = layout_file.as_ref().and_then(|p| std::fs::read_to_string(p).ok()) {
        if let Err(e) = ui.restore_layout(&text) {
            eprintln!("  warning: ui layout file ignored: {e}");
        }
    }
    let keys_file = if args.iter().any(|a| a == "--autotest") { None } else { player_file("keybinds.toml") };
    if let Some(text) = keys_file.as_ref().and_then(|p| std::fs::read_to_string(p).ok()) {
        if let Err(e) = ui.restore_keybinds(&text) {
            eprintln!("  warning: keybinds file ignored: {e}");
        }
    }
    eprintln!("rim: seed {seed}, {} mods loaded, UI font {}", sim.mods.len(), ui.info.font);
    for w in sim.warnings.iter().chain(&ui.warnings()) {
        eprintln!("  warning: {w}");
    }

    let world_atlas = match atlas::WorldAtlas::load(&sim.world.defs.sprite_files) {
        Ok(a) => a,
        Err(e) => return fail(e).await,
    };
    let atlas = Texture2D::from_rgba8(ui.text.atlas.size as u16, ui.text.atlas.size as u16, &ui.text.atlas.pixels);
    atlas.set_filter(FilterMode::Linear);
    let center = sim.world.colony_center().unwrap_or(IVec::new(100, 100));
    let mut app = App {
        tools: toolbar(&sim),
        sim,
        ui,
        atlas,
        cam: Cam { x: center.x as f32 + 0.5, y: center.y as f32 + 0.5, zoom: 28.0 },
        tool: Tool::Select,
        selected: None,
        drag_start: None,
        paused: false,
        speed: 1,
        show_profiler: false,
        show_devtools: false,
        overlay: None,
        hint: None,
        stuff_for: Vec::new(),
        hint_key: None,
        order_flash: None,
        mouse_over_ui: false,
        last_draw: Vec::new(),
        profile: (Vec::new(), Vec::new(), f64::MIN),
        acc: 0.0,
        pan_anchor: None,
        sky: sky::Sky::default(),
        ground: draw::Ground::default(),
        render_us: RenderTimes::default(),
        meshes: mesh::Meshes::default(),
        world_atlas,
        wheel_sub: macroquad::input::utils::register_input_subscriber(),
    };
    app.selected = app.sim.world.colonists().next();

    if let Some(i) = args.iter().position(|a| a == "--autotest") {
        let dir = args.get(i + 1).filter(|a| !a.starts_with("--")).map_or("target/autotest".into(), PathBuf::from);
        autotest::run(app, dir).await;
    }
    if bench {
        bench::run(app, &args).await;
    }

    loop {
        let raw = RawInput::gather(&mut app);
        frame(&mut app, &raw);
        if app.ui.take_layout_dirty() {
            if let Some(p) = &layout_file {
                let _ = p.parent().map(std::fs::create_dir_all);
                if let Err(e) = std::fs::write(p, app.ui.layout_toml()) {
                    eprintln!("rim: could not save the UI layout to {}: {e}", p.display());
                }
            }
        }
        if app.ui.take_keys_dirty() {
            if let Some(p) = &keys_file {
                let _ = p.parent().map(std::fs::create_dir_all);
                if let Err(e) = std::fs::write(p, app.ui.keybinds_toml()) {
                    eprintln!("rim: could not save the keybinds to {}: {e}", p.display());
                }
            }
        }
        render(&mut app);
        next_frame().await
    }
}

/// Look for `mods/` next to the working directory or the executable.
fn find_mods() -> Option<PathBuf> {
    let mut starts = vec![std::env::current_dir().ok()?];
    if let Ok(exe) = std::env::current_exe() {
        starts.push(exe);
    }
    for s in starts {
        for dir in s.ancestors() {
            let m = dir.join("mods");
            if m.join("core/mod.toml").is_file() {
                return Some(m);
            }
        }
    }
    None
}

/// The toolbar is generated from defs: a mod that adds a designation or a
/// buildable thing gets a button without touching the client.
fn toolbar(sim: &Sim) -> Vec<ToolDef> {
    let defs = &sim.world.defs;
    let mut items = vec![ToolDef { key: "select".into(), label: "Select".into(), tool: Tool::Select, color: GRAY }];
    for (i, d) in defs.designations.iter().enumerate() {
        items.push(ToolDef {
            key: format!("designate:{}", d.id),
            label: d.label.clone(),
            tool: Tool::Designate(i as DefId),
            color: rgb(d.rgb),
        });
    }
    let mut builds: Vec<(usize, &rim_sim::defs::ThingDef)> =
        defs.things.iter().enumerate().filter(|(_, t)| t.build.is_some()).collect();
    builds.sort_by_key(|(i, t)| (t.build.as_ref().unwrap().menu.clone(), *i));
    for (i, t) in builds {
        items.push(ToolDef {
            key: format!("build:{}", t.id),
            label: t.label.clone(),
            tool: Tool::Build(i as DefId),
            color: rgb(t.rgb),
        });
    }
    items.push(ToolDef {
        key: "cancel".into(),
        label: "Cancel".into(),
        tool: Tool::Cancel,
        color: Color::from_rgba(200, 80, 80, 255),
    });
    items
}

pub fn rgb(c: [u8; 3]) -> Color {
    Color::from_rgba(c[0], c[1], c[2], 255)
}

fn to_u8(c: Color) -> [u8; 3] {
    [(c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8]
}

/// One frame's raw input, in logical points. The real loop gathers it from
/// macroquad; `--autotest` builds it by hand, so both drive the same path.
/// Sums every wheel event in a frame. `mouse_wheel()` keeps only the last
/// one, and a trackpad sends several per frame.
struct Wheel(f32);

impl macroquad::miniquad::EventHandler for Wheel {
    fn update(&mut self) {}
    fn draw(&mut self) {}
    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        self.0 += y;
    }
}

impl Wheel {
    /// Wheel movement this frame in mouse-wheel notches: one click of a
    /// wheel is 1, a trackpad gives fractions. Backends report notches in
    /// different units.
    fn gather(sub: usize) -> f32 {
        let mut w = Wheel(0.0);
        macroquad::input::utils::repeat_all_miniquad_input(&mut w, sub);
        let per_notch = if cfg!(target_os = "macos") {
            10.0
        } else if cfg!(target_os = "windows") {
            120.0
        } else {
            1.0
        };
        (w.0 / per_notch).clamp(-4.0, 4.0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct RawInput {
    pub mouse: (f32, f32),
    pub left_pressed: bool,
    pub left_released: bool,
    pub right_pressed: bool,
    /// Wheel movement in notches (fractional on a trackpad).
    pub wheel: f32,
    pub keys: Vec<KeyCode>,
    /// Characters typed this frame, for a focused text input.
    pub chars: Vec<char>,
    /// Every key pressed this frame by name, with modifiers ("ctrl+k").
    pub pressed: Vec<String>,
    pub shift: bool,
    /// Camera pan this frame, in tiles (WASD, middle-drag).
    pub pan: (f32, f32),
    pub time: f64,
    /// Advance the simulation by real frame time (the autotest steps it
    /// explicitly instead).
    pub advance: bool,
}

impl RawInput {
    fn gather(app: &mut App) -> RawInput {
        let (mx, my) = mouse_position();
        let keys = [
            KeyCode::Escape,
            KeyCode::Tab,
            KeyCode::Enter,
            KeyCode::Backspace,
            KeyCode::Delete,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Home,
            KeyCode::End,
        ]
        .into_iter()
        .filter(|k| is_key_pressed(*k))
        .collect();
        let mut chars = Vec::new();
        while let Some(c) = get_char_pressed() {
            if let Some(c) = typed_char(c) {
                chars.push(c);
            }
        }
        let ctrl = is_key_down(KeyCode::LeftControl)
            || is_key_down(KeyCode::RightControl)
            || is_key_down(KeyCode::LeftSuper)
            || is_key_down(KeyCode::RightSuper);
        let alt = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
        let shift_down = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        let mut pressed = Vec::new();
        for (code, name) in KEY_NAMES {
            if is_key_pressed(*code) {
                let mut s = String::new();
                for (on, m) in [(ctrl, "ctrl+"), (alt, "alt+"), (shift_down, "shift+")] {
                    if on {
                        s.push_str(m);
                    }
                }
                s.push_str(name);
                pressed.push(s);
            }
        }

        let speed = 18.0 * frame_time() * 40.0 / app.cam.zoom;
        let (mut dx, mut dy) = (0.0, 0.0);
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
            dy -= speed;
        }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
            dy += speed;
        }
        if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
            dx -= speed;
        }
        if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
            dx += speed;
        }
        // Middle-drag pans: the grabbed world point follows the mouse.
        if is_mouse_button_pressed(MouseButton::Middle) {
            app.pan_anchor = Some((mx, my));
        }
        if let Some((ax, ay)) = app.pan_anchor {
            if is_mouse_button_down(MouseButton::Middle) {
                dx -= (mx - ax) / app.cam.zoom;
                dy -= (my - ay) / app.cam.zoom;
                app.pan_anchor = Some((mx, my));
            } else {
                app.pan_anchor = None;
            }
        }
        let wheel = Wheel::gather(app.wheel_sub);
        RawInput {
            mouse: (mx, my),
            left_pressed: is_mouse_button_pressed(MouseButton::Left),
            left_released: is_mouse_button_released(MouseButton::Left),
            right_pressed: is_mouse_button_pressed(MouseButton::Right),
            wheel,
            keys,
            chars,
            pressed,
            shift: is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift),
            pan: (dx, dy),
            time: get_time(),
            advance: true,
        }
    }
}

/// What the UI reads each frame.
pub fn client_view(app: &mut App, mouse: (f32, f32), time: f64) -> ClientView {
    let dpi = screen_dpi_scale();
    let s = &app.sim;
    // Profiler rows refresh a few times a second: a number changing every
    // frame would reflow the panel every frame.
    if app.show_profiler && time - app.profile.2 >= 0.25 {
        let w = &s.world;
        let (hooks, handlers) = s.scripts.hook_count();
        let mut rows = s.profile.entries.clone();
        for (m, us) in &app.ui.vm.mod_time {
            rows.push((format!("ui:{m}"), *us));
        }
        for (pass, us) in app.render_us.rows() {
            rows.push((format!("draw:{pass}"), us));
        }
        app.profile = (
            rows,
            vec![
                format!("tick {} · pawns {} · entities {}", w.tick, w.pawns.len(), w.ecs.len()),
                format!("paths {} · nodes {} · reservations {}", w.pf.searches, w.pf.expanded, w.reservations.len()),
                format!("script hooks {hooks} · handlers {handlers} · emitters {}", w.fields.emitter_count()),
                format!(
                    "chunk meshes: {} calls · {}k indices · {} rebuilt",
                    app.meshes.calls,
                    app.meshes.indices / 1000,
                    app.meshes.rebuilt
                ),
            ],
            time,
        );
    }
    let hover_cell = (!app.mouse_over_ui).then(|| app.cam.tile_at(mouse.0, mouse.1));
    let hover_pawn = if app.mouse_over_ui { None } else { pawn_under(app, mouse.0, mouse.1) };
    let drag = app.drag_start.map(|a| {
        let b = app.cam.tile_at(mouse.0, mouse.1);
        format!("{} × {}", (a.x - b.x).abs() + 1, (a.y - b.y).abs() + 1)
    });
    ClientView {
        screen: (screen_width() * dpi, screen_height() * dpi),
        scale: app.ui.theme.scale,
        cam: (app.cam.x, app.cam.y, app.cam.zoom * dpi),
        mouse: (mouse.0 * dpi, mouse.1 * dpi),
        selected: app.selected,
        paused: app.paused,
        speed: app.speed,
        overlay: app.overlay,
        show_profiler: app.show_profiler,
        show_devtools: app.show_devtools,
        tools: app
            .tools
            .iter()
            .map(|t| ToolView {
                key: t.key.clone(),
                label: t.label.clone(),
                color: to_u8(t.color),
                active: t.tool == app.tool,
            })
            .collect(),
        stuff: stuff_view(app),
        hint: drag.or_else(|| app.hint.clone()),
        hover_cell,
        hover_pawn,
        time,
        profile: app.profile.0.clone(),
        stats: app.profile.1.clone(),
        mods: s.mods.iter().map(|m| (m.id.clone(), m.version.clone(), m.name.clone())).collect(),
        warnings: s.warnings.iter().cloned().chain(app.ui.warnings()).collect(),
    }
}

/// One frame of input: UI first, then the world gets what the UI didn't take.
pub fn frame(app: &mut App, raw: &RawInput) {
    let dpi = screen_dpi_scale();
    app.ui.set_dpi(dpi);
    app.ui.check_reload(raw.time);
    let cv = client_view(app, raw.mouse, raw.time);
    let has = |k: KeyCode| raw.keys.contains(&k);
    let input = rim_ui::Input {
        mouse: (raw.mouse.0 * dpi, raw.mouse.1 * dpi),
        left_pressed: raw.left_pressed,
        left_released: raw.left_released,
        right_pressed: raw.right_pressed,
        wheel: raw.wheel,
        tab: has(KeyCode::Tab),
        shift: raw.shift,
        enter: has(KeyCode::Enter),
        pressed: raw.pressed.clone(),
        keys: {
            use rim_ui::Key;
            let mut keys: Vec<Key> = raw.chars.iter().map(|&c| Key::Char(c)).collect();
            for (code, key) in [
                (KeyCode::Backspace, Key::Backspace),
                (KeyCode::Delete, Key::Delete),
                (KeyCode::Left, Key::Left),
                (KeyCode::Right, Key::Right),
                (KeyCode::Up, Key::Up),
                (KeyCode::Down, Key::Down),
                (KeyCode::Home, Key::Home),
                (KeyCode::End, Key::End),
                (KeyCode::Escape, Key::Escape),
            ] {
                if has(code) {
                    keys.push(key);
                }
            }
            keys
        },
        time: raw.time,
    };
    let out = app.ui.frame(&app.sim.world, &cv, &input);
    app.last_draw = out.draw;
    app.mouse_over_ui = out.mouse_over_ui;
    for a in out.actions {
        apply_ui(app, a);
    }

    // Keys, unless the UI used them (a focused text input takes them all;
    // Tab/Enter go to a focused control).
    // Everything else with a key is a core binding (mods/core/ui/keys.luau).
    for k in raw.keys.iter().filter(|_| !out.captured_keys) {
        let action = match k {
            KeyCode::Escape => {
                app.ui.blur();
                Action::Escape
            }
            KeyCode::Tab => Action::NextColonist,
            _ => continue,
        };
        apply(app, action);
    }
    if raw.pan != (0.0, 0.0) && !out.captured_keys {
        apply(app, Action::Pan(raw.pan.0, raw.pan.1));
    }
    let (mx, my) = raw.mouse;
    if raw.wheel != 0.0 && !out.captured_wheel {
        // Proportional, so a trackpad zooms smoothly and a wheel click is
        // one 12% step.
        apply(app, Action::Zoom(1.12f32.powf(raw.wheel), mx, my));
    }
    if raw.left_pressed && !out.captured_left {
        apply(app, Action::LeftDown(mx, my));
    }
    if raw.left_released && app.drag_start.is_some() {
        apply(app, Action::LeftUp(mx, my));
    }
    if raw.right_pressed && !out.captured_right {
        apply(app, Action::RightClick(mx, my));
    }

    if raw.advance {
        step(app);
    }
    hint(app, raw.mouse);
}

/// CPU time of each render pass last frame, in µs. This is building the
/// batches; the GL work happens when the frame ends.
#[derive(Clone, Copy, Default, Debug)]
pub struct RenderTimes {
    pub ground: f64,
    /// Painting, excluding `gl`.
    pub things: f64,
    /// Handing the chunk meshes (and the batch before each layer) to GL
    /// mid-frame. Submission, like macroquad's end of frame: a software
    /// rasteriser does its drawing here, a GPU driver only queues.
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

pub fn render(app: &mut App) {
    let mut clock = std::time::Instant::now();
    let mut lap = || {
        let us = clock.elapsed().as_secs_f64() * 1e6;
        clock = std::time::Instant::now();
        us
    };
    let mut t = RenderTimes::default();
    app.ground.update(&app.sim.world);
    t.ground = lap();
    let counts = draw::things(app);
    t.gl = app.meshes.submit_us;
    t.things = lap() - t.gl;
    draw::pawns(app);
    t.pawns = lap();
    let air = sky::Air::read(&app.sim.world);
    app.sky.weather(&app.sim.world, &app.cam, &air);
    t.weather = lap();
    app.sky.light(&app.sim.world, &app.cam, &air);
    t.light = lap();
    draw::world_ui(app);
    // Stack counts, in the UI's text: shaped into the same atlas, drawn in
    // the same batch as the UI. After lighting, so they read at night.
    let dpi = screen_dpi_scale();
    let mut labels = Vec::with_capacity(counts.len() * 2);
    for (x, y, n) in counts {
        let text = n.to_string();
        for (dx, color) in [(1.0, [0.0, 0.0, 0.0, 0.6]), (0.0, [1.0, 1.0, 1.0, 1.0])] {
            let quads = app.ui.text.quads(&text, 13.0 * dpi, 600, None, (x + dx) * dpi, (y + dx) * dpi);
            labels.push(rim_ui::paint::Draw::Glyphs { quads, color });
        }
    }
    if app.ui.text.atlas.dirty {
        let a = &app.ui.text.atlas;
        app.atlas.update(&Image { bytes: a.pixels.clone(), width: a.size as u16, height: a.size as u16 });
        app.ui.text.atlas.dirty = false;
    }
    let white = app.ui.text.atlas.white_texel();
    draw::ui(&labels, &app.atlas, white, dpi);
    draw::ui(&app.last_draw, &app.atlas, white, dpi);
    t.ui = lap();
    app.render_us = t;
}

fn apply_ui(app: &mut App, a: UiAction) {
    match a {
        UiAction::Select(e) => app.selected = e,
        UiAction::Focus(e) => focus(app, e),
        UiAction::Tool(key) => {
            if let Some(t) = app.tools.iter().find(|t| t.key == key) {
                app.tool = t.tool;
                app.drag_start = None;
            }
        }
        UiAction::Stuff(id) => {
            if let (Tool::Build(t), Some(m)) = (app.tool, app.sim.world.defs.thing_id(&id)) {
                let sc = app.sim.world.defs.thing(t).build.as_ref().and_then(|b| b.stuff.as_ref());
                if sc.is_some_and(|sc| app.sim.world.defs.is_material_for(m, &sc.category)) {
                    app.stuff_for.retain(|(b, _)| *b != t);
                    app.stuff_for.push((t, m));
                }
            }
        }
        UiAction::Speed(s) => apply(app, Action::Speed(s)),
        UiAction::TogglePause => apply(app, Action::TogglePause),
        UiAction::Draft(e, on) => app.sim.push(Command::Draft { pawn: e, on }),
        UiAction::CycleOverlay => apply(app, Action::CycleOverlay),
        UiAction::SetOverlay(o) => app.overlay = o.filter(|i| *i < app.sim.world.defs.fields.len()),
        UiAction::ToggleProfiler => apply(app, Action::ToggleProfiler),
        UiAction::ToggleDevtools => apply(app, Action::ToggleDevtools),
        UiAction::ToggleOutlines => app.ui.toggle_outlines(),
        UiAction::Send(name, data) => app.sim.push(Command::ModEvent { name, data }),
        UiAction::Advance(hours) => {
            // Devtools only: step the sim now, as fast as it goes.
            let ticks = (hours * rim_sim::TICKS_PER_DAY as f64 / 24.0) as u64;
            for _ in 0..ticks {
                app.sim.step();
            }
        }
    }
}

fn step(app: &mut App) {
    if app.paused || app.sim.world.colony_lost {
        app.acc = 0.0;
        return;
    }
    app.acc += frame_time() as f64 * 60.0 * app.speed as f64;
    let budget = std::time::Instant::now();
    while app.acc >= 1.0 {
        app.sim.step();
        app.acc -= 1.0;
        if budget.elapsed().as_millis() > 12 {
            app.acc = app.acc.min(4.0);
            break;
        }
    }
    if app.selected.is_some_and(|e| !app.sim.world.pawn_alive(e)) {
        app.selected = None;
    }
}

/// Label the cursor with what a right-click would do. Resolving an order
/// walks the map, so it happens once per tile rather than once per frame.
fn hint(app: &mut App, mouse: (f32, f32)) {
    let (mx, my) = mouse;
    let Some(e) = app.selected.filter(|_| app.tool == Tool::Select && !app.mouse_over_ui) else {
        app.hint = None;
        app.hint_key = None;
        return;
    };
    let key = (e, app.cam.tile_at(mx, my), pawn_under(app, mx, my));
    if app.hint_key == Some(key) {
        return;
    }
    app.hint_key = Some(key);
    app.hint = order::resolve(&app.sim.world, e, key.1, key.2).map(|o| o.label);
}

pub fn pawn_under(app: &App, sx: f32, sy: f32) -> Option<Entity> {
    let (wx, wy) = app.cam.to_world(sx, sy);
    let w = &app.sim.world;
    let mut best: Option<(f32, Entity)> = None;
    for &e in &w.pawns {
        let Ok(p) = w.ecs.get::<&Pawn>(e) else { continue };
        let (px, py) = draw::pawn_pos(&p);
        let d = ((px - wx).powi(2) + (py - wy).powi(2)).sqrt();
        let bias = if p.faction == Faction::Player { -0.2 } else { 0.0 };
        if d < 0.7 && best.is_none_or(|b| d + bias < b.0) {
            best = Some((d + bias, e));
        }
    }
    best.map(|b| b.1)
}

/// Everything the player can do to the world, independent of which key or
/// button did it. UI controls produce `UiAction`s instead.
#[derive(Clone, Copy, Debug)]
pub enum Action {
    TogglePause,
    Speed(u32),
    ToggleProfiler,
    ToggleDevtools,
    /// Cycle the field overlay: off, then each field the mods define.
    CycleOverlay,
    Escape,
    ToggleDraft,
    NextColonist,
    CenterSelected,
    /// Move the camera by this many tiles.
    Pan(f32, f32),
    /// Zoom by a factor, keeping the world point under (x, y) fixed.
    Zoom(f32, f32, f32),
    /// Left button went down / up on the world at a screen position.
    LeftDown(f32, f32),
    LeftUp(f32, f32),
    RightClick(f32, f32),
}

pub fn apply(app: &mut App, action: Action) {
    match action {
        Action::TogglePause => app.paused = !app.paused,
        Action::Speed(s) => {
            app.speed = s;
            app.paused = false;
        }
        Action::ToggleProfiler => app.show_profiler = !app.show_profiler,
        Action::ToggleDevtools => {
            app.show_devtools = !app.show_devtools;
            app.ui.devtools = app.show_devtools;
        }
        Action::CycleOverlay => {
            // Only fields that vary over the map have an overlay.
            let fields = &app.sim.world.defs.fields;
            let from = app.overlay.map_or(0, |i| i + 1);
            app.overlay = (from..fields.len()).find(|&i| fields[i].overlay);
        }
        Action::Escape => {
            if app.tool != Tool::Select {
                app.tool = Tool::Select;
            } else {
                app.selected = None;
            }
            app.drag_start = None;
        }
        Action::ToggleDraft => {
            if let Some(e) = app.selected {
                let drafted = app.sim.world.ecs.get::<&Pawn>(e).map(|p| (p.faction == Faction::Player, p.drafted)).ok();
                if let Some((true, d)) = drafted {
                    app.sim.push(Command::Draft { pawn: e, on: !d });
                }
            }
        }
        Action::NextColonist => {
            let cols: Vec<Entity> = app.sim.world.colonists().collect();
            if !cols.is_empty() {
                let i =
                    app.selected.and_then(|s| cols.iter().position(|&c| c == s)).map_or(0, |i| (i + 1) % cols.len());
                app.selected = Some(cols[i]);
                focus(app, cols[i]);
            }
        }
        Action::CenterSelected => {
            if let Some(e) = app.selected {
                focus(app, e);
            }
        }
        Action::Pan(dx, dy) => {
            app.cam.x += dx;
            app.cam.y += dy;
        }
        Action::Zoom(f, x, y) => {
            let before = app.cam.to_world(x, y);
            app.cam.zoom = (app.cam.zoom * f).clamp(MIN_ZOOM, 80.0);
            let after = app.cam.to_world(x, y);
            app.cam.x += before.0 - after.0;
            app.cam.y += before.1 - after.1;
        }
        Action::LeftDown(x, y) => match app.tool {
            Tool::Select => app.selected = pawn_under(app, x, y),
            _ => app.drag_start = Some(app.cam.tile_at(x, y)),
        },
        Action::LeftUp(x, y) => {
            let Some(a) = app.drag_start.take() else { return };
            let b = app.cam.tile_at(x, y);
            let defs = app.sim.world.defs.clone();
            match app.tool {
                Tool::Designate(d) => {
                    // A click on a creature uses a generous box so moving targets are caught.
                    let (a, b) = if defs.designations[d as usize].targets == Targets::Creature && a == b {
                        (a.offset(-1, -1), b.offset(1, 1))
                    } else {
                        (a, b)
                    };
                    app.sim.push(Command::Designate { designation: d, a, b });
                }
                Tool::Build(t) => {
                    let stuff = chosen_material(app, t);
                    for (a, b) in build_rects(defs.thing(t).blocks, a, b) {
                        app.sim.push(Command::Build { stuff, thing: t, a, b });
                    }
                }
                Tool::Cancel => app.sim.push(Command::Cancel { a, b }),
                Tool::Select => {}
            }
        }
        Action::RightClick(x, y) => {
            if app.tool != Tool::Select {
                app.tool = Tool::Select;
                app.drag_start = None;
                return;
            }
            let Some(e) = app.selected else { return };
            let cell = app.cam.tile_at(x, y);
            let on = pawn_under(app, x, y);
            if order::resolve(&app.sim.world, e, cell, on).is_none() {
                return;
            }
            app.sim.push(Command::Order { pawn: e, cell, on });
            app.order_flash = Some((cell, get_time()));
        }
    }
    let (mw, mh) = (app.sim.world.map.w as f32, app.sim.world.map.h as f32);
    app.cam.x = app.cam.x.clamp(0.0, mw);
    app.cam.y = app.cam.y.clamp(0.0, mh);
}

/// Walls (anything that blocks) are drawn as a room outline; everything
/// else fills the dragged rectangle.
/// How much of an item the colony has lying around, blueprints aside.
fn stock(w: &rim_sim::world::World, d: DefId) -> u32 {
    w.ecs
        .query::<&rim_sim::world::Thing>()
        .without::<&rim_sim::world::Blueprint>()
        .iter()
        .filter(|t| t.def == d)
        .map(|t| t.count)
        .sum::<u32>()
}

/// The material `thing` will be built from: what the player last picked
/// for it, else whatever the colony has most of, else the first the def
/// would accept so a blueprint can still be placed and waited on.
fn chosen_material(app: &App, thing: DefId) -> Option<DefId> {
    let w = &app.sim.world;
    let sc = w.defs.thing(thing).build.as_ref()?.stuff.as_ref()?;
    let options = w.defs.materials(&sc.category);
    if let Some((_, m)) = app.stuff_for.iter().find(|(b, _)| *b == thing) {
        if options.contains(m) {
            return Some(*m);
        }
    }
    options.iter().copied().max_by_key(|&d| stock(w, d)).or_else(|| options.first().copied())
}

/// The material row for the active build tool: every material its def
/// accepts, with stock and what the result would be, or nothing at all.
fn stuff_view(app: &App) -> Vec<rim_ui::view::StuffView> {
    let Tool::Build(t) = app.tool else { return Vec::new() };
    let w = &app.sim.world;
    let td = w.defs.thing(t);
    let Some(b) = td.build.as_ref() else { return Vec::new() };
    let Some(sc) = b.stuff.as_ref() else { return Vec::new() };
    let active = chosen_material(app, t);
    w.defs
        .materials(&sc.category)
        .into_iter()
        .map(|m| {
            let md = w.defs.thing(m);
            rim_ui::view::StuffView {
                id: md.id.clone(),
                label: md.label.clone(),
                color: md.rgb,
                have: stock(w, m),
                active: active == Some(m),
                hp: (td.hp as f64 * w.defs.factor(Some(m), "hp")).round() as u32,
                work: (b.work as f64 * w.defs.factor(Some(m), "work")).round() as u32,
            }
        })
        .collect()
}

pub fn build_rects(blocks: bool, a: IVec, b: IVec) -> Vec<(IVec, IVec)> {
    let (x0, x1, y0, y1) = (a.x.min(b.x), a.x.max(b.x), a.y.min(b.y), a.y.max(b.y));
    if !blocks || x1 - x0 < 2 || y1 - y0 < 2 {
        return vec![(a, b)];
    }
    vec![
        (IVec::new(x0, y0), IVec::new(x1, y0)),
        (IVec::new(x0, y1), IVec::new(x1, y1)),
        (IVec::new(x0, y0 + 1), IVec::new(x0, y1 - 1)),
        (IVec::new(x1, y0 + 1), IVec::new(x1, y1 - 1)),
    ]
}

fn focus(app: &mut App, e: Entity) {
    if let Ok(p) = app.sim.world.ecs.get::<&Pawn>(e) {
        app.cam.x = p.pos.x as f32 + 0.5;
        app.cam.y = p.pos.y as f32 + 0.5;
    }
}
