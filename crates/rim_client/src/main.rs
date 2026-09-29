//! rim client: renders the simulation and turns input into Commands.
//! The client never mutates the world directly.
//!
//! Each frame: raw input goes to the UI first (rim_ui routes it against the
//! last layout; panels swallow clicks). UI actions apply, then whatever the
//! UI didn't take drives the world. The HUD itself is core's UI mod.

mod actions;
mod atlas;
mod autotest;
mod bench;
mod cli;
mod draw;
mod figures;
mod frames;
mod grid;
mod input;
mod light;
mod mesh;
mod motion;
mod occluders;
mod overlay;
mod pattern;
mod pinch;
mod quality;
mod quiet;
mod render;
mod roof;
mod roomstate;
mod save;
mod seedcmd;
mod settings;
mod sky;
mod title;
mod tools;
mod wear;
mod worksite;

pub(crate) use actions::*;
pub(crate) use input::*;
pub use render::{render, upload_atlas, RenderTimes};
pub(crate) use settings::*;
pub(crate) use tools::*;

use macroquad::prelude::*;
use rim_sim::defs::{DefId, Targets};
use rim_sim::hecs::Entity;
use rim_sim::order;
use rim_sim::world::{Faction, Pawn};
use rim_sim::{Command, IVec, Sim};
use rim_ui::view::{ClientView, ToolView, UiAction};
use rim_ui::Ui;
use std::path::PathBuf;

/// The lowest zoom, in points per tile: a 250-cell map fits a 1080p screen.
pub const MIN_ZOOM: f32 = 4.0;

pub struct Cam {
    /// Center of the view, in tiles.
    pub x: f32,
    pub y: f32,
    /// Logical points per tile.
    pub zoom: f32,
    /// The level on screen (DESIGN.md §6d): 0 is the surface.
    pub z: i32,
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
        IVec::at(wx.floor() as i32, wy.floor() as i32, self.z)
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
    /// The buildable the build tray is describing, when it isn't the tool
    /// in hand: its materials go to the UI in place of the tool's.
    pub preview: Option<DefId>,
    /// Quarter turns clockwise for what the build tool places.
    pub build_facing: u8,
    /// What the inspector shows: the one selected thing, or the first of
    /// `group`.
    pub selected: Option<Entity>,
    /// A stockpile the inspector shows, when nothing else is selected.
    pub selected_zone: Option<u32>,
    /// Every selected colonist when more than one is; empty otherwise.
    pub group: Vec<Entity>,
    /// Shift is held this frame: a click adds to the selection.
    pub shift: bool,
    /// Alt, Ctrl or Cmd is held this frame: a box takes colonists out
    /// (Ctrl too, since many Linux desktops keep Alt-drag to move windows).
    pub subtract: bool,
    pub drag_start: Option<IVec>,
    /// Where a drag began on screen, to tell a click from a box.
    pub drag_from: (f32, f32),
    pub paused: bool,
    pub speed: u32,
    pub show_profiler: bool,
    pub show_devtools: bool,
    /// The last second's frames, for the profiler's header.
    pub frames: frames::FrameStats,
    /// Field layer drawn over the map, if any (cycled with O).
    pub overlay: Option<usize>,
    /// The storage overlay: the last stop of the O cycle, after the fields.
    pub storage_overlay: bool,
    /// What a right-click would do, recomputed only when the cursor moves to
    /// another tile or the selection changes.
    pub hint: Option<String>,
    hint_key: Option<(Entity, IVec, Option<Entity>)>,
    /// Where the last order landed, and when, for a brief marker.
    pub order_flash: Option<(IVec, f64)>,
    /// The pointer was over UI last frame (world hover is suppressed).
    pub mouse_over_ui: bool,
    /// The world cell under the pointer, when it isn't over the UI.
    pub hover_cell: Option<IVec>,
    /// The UI's draw list from the last frame.
    pub last_draw: Vec<rim_ui::paint::Draw>,
    /// The anchored labels among `last_draw`, to catch up with the camera
    /// and the pawns before they're drawn.
    pub last_anchored: Vec<rim_ui::AnchoredDraw>,
    /// Profiler rows, refreshed a few times a second.
    profile: (Vec<(String, f64)>, Vec<String>, f64),
    acc: f64,
    pan_anchor: Option<(f32, f32)>,
    /// A right press on the map, until it's decided: a click, a hold that
    /// opens the orders menu, or a drag.
    right: Option<RightPress>,
    /// What a scroll does on the map (the player's `scroll` setting).
    pub scroll_mode: ScrollMode,
    /// When the last precise (trackpad) scroll came.
    last_precise: f64,
    /// The last order given, so it can be taken back.
    pub last_order: Option<LastOrder>,
    /// Weather on screen.
    pub sky: sky::Sky,
    /// Roofs from far away, rebuilt when rooms are.
    pub roofs: roof::Roofs,
    /// Gaps and open sky, rebuilt when rooms are.
    pub marks: roomstate::RoomMarks,
    /// Light on screen: shadows, firelight, and the multiply over the world.
    pub light: light::Light,
    /// The terrain, baked into a texture.
    pub ground: draw::Ground,
    pub water: draw::Water,
    /// What each render pass cost last frame.
    pub render_us: RenderTimes,
    /// Floors, items and fixtures, cached per chunk on the GPU.
    pub meshes: mesh::Meshes,
    /// Blows and finishes on the sites being worked, followed frame to frame.
    pub worksites: worksite::Worksites,
    /// How pawns are drawn moving: the cell each came from, and facing.
    pub motion: motion::Motion,
    /// Every pawn's figure, drawn in one call (DESIGN.md §6h).
    pub figures: figures::Batch,
    /// The pawns drawn this frame and where, for their marks.
    pub figures_shown: Vec<(Entity, (f32, f32))>,
    /// Every mod's sprites, packed at load.
    pub world_atlas: atlas::WorldAtlas,
    /// The world's resolution as a fraction of the screen's pixels, if
    /// the player chose one; unset is full. The UI is always full. The
    /// world draws into `world_target` (see `update_world_target`).
    pub render_scale: Option<f32>,
    pub world_target: Option<RenderTarget>,
    /// Puts `world_target` on the screen without blending: what the world
    /// left in its alpha is not transparency.
    blit: Option<Material>,
    /// Changing level, the last frame of the level left, fading out over
    /// the new one, and how far it has (0 to 1).
    fade: Option<(RenderTarget, f32)>,
    /// The level the last frame drew, to know when it changes.
    drawn_z: i32,
    /// Draws `fade`'s frame at an alpha, its own alpha ignored as `blit`'s is.
    fade_blit: Option<Material>,
    /// Where the player's settings are saved; none in the autotest.
    settings_file: Option<PathBuf>,
    /// Input subscriber for wheel events (see `Wheel`).
    wheel_sub: usize,
    /// The save this game appends to, if it's being saved.
    pub saver: Option<rim_sim::savefile::Writer>,
    /// Overlay colours and sizes, from the theme each frame (DESIGN.md §6f).
    pub palette: overlay::Palette,
    pub chalk: overlay::State,
    /// The grid's fades and the pointer it follows (DESIGN.md §6f).
    pub grid: grid::Grid,
    /// The measuring grid is on (G).
    pub measure: bool,
    /// The player asked for a still map: overlays appear and go at once.
    pub reduce_motion: bool,
    /// What a stockpile or clear-zone drag will change, made once a frame.
    pub zone_preview: Option<draw::ZonePreview>,
    /// What a designate or cancel tool would do, made once a frame.
    pub order_preview: Option<overlay::OrderPreview>,
    /// What a build tool would put up, made once a frame.
    pub build_preview: Option<overlay::BuildPreview>,
    /// A click that did nothing: why, where, and when.
    pub refused: Option<(String, IVec, f64)>,
    /// The pointer as this frame's input had it, in screen points. Previews
    /// follow it rather than the OS cursor, so replayed input (the
    /// autotest) draws what it did.
    pub pointer: (f32, f32),
    /// The frame's clock, from its input: everything drawn over time reads
    /// these, never macroquad's clock, so replayed input (the autotest)
    /// draws the same frames on any machine.
    pub now: f64,
    /// Seconds since the last frame, clamped as `frame_time` is.
    pub dt: f32,
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
                // with the display itself and ignores this); off for the
                // render bench, which measures a frame's cost, not the display.
                swap_interval: Some(if std::env::args().any(|a| a == "--bench-render") { 0 } else { 1 }),
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
    // Nobody is watching the window in the autotest or the bench: say it
    // and exit, so CI fails instead of waiting forever.
    if std::env::args().any(|a| a == "--autotest" || a == "--bench-render") {
        eprintln!("rim failed to start: {e}");
        std::process::exit(2);
    }
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
        Some("replay") => std::process::exit(cli::replay(&args[2..])),
        Some("save") => std::process::exit(cli::save(&args[2..])),
        Some("seeds") => std::process::exit(seedcmd::seeds(&args[2..])),
        _ => {}
    }
    // A test run, or a game started for someone who is doing something
    // else, opens without taking them to its Space.
    if args.iter().any(|a| a == "--background" || a == "--autotest" || a == "--bench-render") {
        quiet::install();
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
    let ui_scale_arg: Option<f32> = args.windows(2).find(|w| w[0] == "--ui-scale").and_then(|w| w[1].parse().ok());

    let bench = args.iter().any(|a| a == "--bench-render");
    // The autotest and the benchmark are tests: they don't touch the saves.
    let saving = !bench && !args.iter().any(|a| a == "--autotest");
    let Some(mods) = find_mods() else { return fail("could not find a mods/ directory".into()).await };
    // The UI comes up before any game, since the title screen is UI.
    let loaded = match rim_sim::modloader::load(&mods) {
        Ok(l) => l,
        Err(e) => return fail(e).await,
    };
    // The player's UI scale, unless the command line names one; the
    // autotest and the benchmark measure at 1.
    let tests = args.iter().any(|a| a == "--autotest" || a == "--bench-render");
    let settings_file = if tests { None } else { player_file("settings.toml") };
    // Read once. No file is the defaults; a file that can't be read is
    // said, then the defaults.
    let settings = settings_file.as_ref().and_then(|p| match std::fs::read_to_string(p) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            eprintln!("  warning: settings file {}: {e}", p.display());
            None
        }
    });
    let ui_scale = ui_scale_arg.unwrap_or_else(|| {
        settings
            .as_deref()
            .map_or(Ok(None), saved_ui_scale)
            .unwrap_or_else(|e| {
                eprintln!("  warning: settings file: {e}");
                None
            })
            .unwrap_or(1.0)
    });
    let mut ui = match Ui::new(rim_ui::vm::mod_dirs(&loaded.mods), screen_dpi_scale(), ui_scale) {
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
    // What a scroll does on the map: the player's setting, else sorted by
    // device. The tests read devices as they come.
    let scroll_mode = settings
        .as_deref()
        .and_then(|text| {
            saved_scroll_mode(text).unwrap_or_else(|e| {
                eprintln!("  warning: settings file: {e}");
                None
            })
        })
        .unwrap_or_default();
    let reduce_motion =
        settings.as_deref().and_then(|text| setting(saved_flag(text, "reduce_motion"))).unwrap_or(false);
    // `--lighting <preset>` for the bench and tests; else the settings file.
    let lighting = match args.windows(2).find(|w| w[0] == "--lighting") {
        Some(w) => quality::Setting::named(&w[1]).unwrap_or_else(|| {
            eprintln!("  warning: --lighting wants low, medium, high, ultra or auto, not {}", w[1]);
            quality::Setting::default()
        }),
        None => settings
            .as_deref()
            .and_then(|text| {
                quality::Setting::from_settings(text).unwrap_or_else(|e| {
                    eprintln!("  warning: settings file: {e}");
                    None
                })
            })
            .unwrap_or_default(),
    };
    let render_scale = if tests {
        // They measure full resolution unless they ask.
        Some(1.0)
    } else {
        match settings.as_deref() {
            Some(text) => saved_render_scale(text).unwrap_or_else(|e| {
                eprintln!("  warning: settings file: {e}");
                None
            }),
            None => None,
        }
    };
    if let Some(text) = keys_file.as_ref().and_then(|p| std::fs::read_to_string(p).ok()) {
        if let Err(e) = ui.restore_keybinds(&text) {
            eprintln!("  warning: keybinds file ignored: {e}");
        }
    }
    let atlas = Texture2D::from_rgba8(ui.text.atlas.size as u16, ui.text.atlas.size as u16, &ui.text.atlas.pixels);
    atlas.set_filter(FilterMode::Linear);
    let wheel_sub = macroquad::input::utils::register_input_subscriber();
    // The window is up: its view can learn to hear a pinch (macOS).
    if cfg!(target_os = "macos") && !pinch::install() {
        eprintln!("  warning: pinch to zoom unavailable; Cmd+scroll zooms");
    }

    // The command line can name the game; otherwise the player picks one.
    let named = |a: &String| matches!(a.as_str(), "--seed" | "--load" | "--continue");
    let opened = if bench {
        let n = match args.iter().position(|a| a == "--sprite-mods") {
            None => Ok(0),
            Some(i) => args
                .get(i + 1)
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| "--sprite-mods wants a number of mods".to_string()),
        };
        n.and_then(|n| bench::world(&mods, seed, n)).map(|s| (s, None, Vec::new()))
    } else if !saving {
        Sim::new(&mods, seed).map(|s| (s, None, Vec::new()))
    } else if args.iter().any(named) {
        save::start_of(&args).and_then(|start| save::open(&mods, seed, start))
    } else {
        title::run(&mut ui, &atlas, wheel_sub, &mods, loaded.defs, seed).await
    };
    let (mut sim, saver, notes) = match opened {
        Ok(x) => x,
        Err(e) => return fail(e).await,
    };
    sim.warnings.extend(notes);
    if saver.is_some() && args.iter().any(|a| a == "--seed") && args.iter().any(|a| a == "--load" || a == "--continue")
    {
        sim.warnings.push("--seed is ignored when loading a save".into());
    }
    eprintln!("rim: seed {}, {} mods loaded, UI font {}", sim.world.seed, sim.mods.len(), ui.info.font);
    if saver.is_some() {
        // Closing the window takes a last snapshot first.
        prevent_quit();
    }
    sim.warnings.extend(figures::unbodied(&ui.bodies, &sim.world.defs));
    for w in sim.warnings.iter().chain(&ui.warnings()) {
        eprintln!("  warning: {w}");
    }

    let world_atlas = match atlas::WorldAtlas::load(&sim.world.defs.sprite_files, &sim.world.defs.glyphs, &mut ui.text)
    {
        Ok(a) => a,
        Err(e) => return fail(e).await,
    };
    let center = sim.world.colony_center().unwrap_or(IVec::new(100, 100));
    let palette = overlay::Palette::from_theme(&ui.theme);
    let mut app = App {
        tools: toolbar(&sim),
        sim,
        ui,
        atlas,
        cam: Cam { x: center.x as f32 + 0.5, y: center.y as f32 + 0.5, zoom: 28.0, z: 0 },
        tool: Tool::Select,
        selected: None,
        selected_zone: None,
        group: Vec::new(),
        shift: false,
        subtract: false,
        drag_start: None,
        drag_from: (0.0, 0.0),
        paused: false,
        speed: 1,
        show_profiler: false,
        show_devtools: false,
        frames: Default::default(),
        overlay: None,
        storage_overlay: false,
        hint: None,
        stuff_for: Vec::new(),
        preview: None,
        build_facing: 0,
        hint_key: None,
        order_flash: None,
        mouse_over_ui: false,
        hover_cell: None,
        last_draw: Vec::new(),
        last_anchored: Vec::new(),
        profile: (Vec::new(), Vec::new(), f64::MIN),
        acc: 0.0,
        pan_anchor: None,
        right: None,
        scroll_mode,
        last_precise: f64::MIN,
        last_order: None,
        sky: sky::Sky::default(),
        roofs: roof::Roofs::default(),
        marks: roomstate::RoomMarks::default(),
        light: light::Light::with(lighting),
        ground: draw::Ground::default(),
        water: draw::Water::default(),
        render_us: RenderTimes::default(),
        meshes: mesh::Meshes::default(),
        worksites: worksite::Worksites::default(),
        motion: motion::Motion::default(),
        figures: figures::Batch::default(),
        figures_shown: Vec::new(),
        world_atlas,
        render_scale,
        world_target: None,
        blit: None,
        fade: None,
        drawn_z: 0,
        fade_blit: None,
        settings_file,
        wheel_sub,
        saver,
        palette,
        chalk: overlay::State::default(),
        grid: grid::Grid::default(),
        pointer: (0.0, 0.0),
        measure: false,
        reduce_motion,
        zone_preview: None,
        order_preview: None,
        build_preview: None,
        refused: None,
        now: 0.0,
        dt: 0.0,
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
        if is_quit_requested() {
            if let Some(w) = app.saver.take() {
                save::close(w, &mut app.sim);
            }
            std::process::exit(0);
        }
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

/// One frame of input: UI first, then the world gets what the UI didn't take.
pub fn frame(app: &mut App, raw: &RawInput) {
    app.dt = ((raw.time - app.now) as f32).clamp(0.0, 0.1);
    app.now = raw.time;
    let dpi = screen_dpi_scale();
    app.shift = raw.shift;
    app.subtract = raw.alt || raw.zoom_mod;
    app.ui.set_dpi(dpi);
    if app.ui.check_reload(raw.time) {
        app.palette = overlay::Palette::from_theme(&app.ui.theme);
    }
    let cv = client_view(app, raw.mouse, raw.time);
    app.hover_cell = cv.hover_cell;
    let input = ui_input(raw, dpi);
    let out = app.ui.frame(&app.sim.world, &cv, &input);
    app.last_draw = out.draw;
    app.last_anchored = out.anchored;
    app.mouse_over_ui = out.mouse_over_ui;
    for a in out.actions {
        apply_ui(app, a);
    }

    let cam_before = (app.cam.x, app.cam.y, app.cam.zoom);
    // Escape always gives the keyboard back; what else it does (close the
    // dock's palette, drop the tool, deselect) is core's `core:escape`
    // binding, so a mod can put its own step in front.
    if raw.keys.contains(&KeyCode::Escape) {
        app.ui.blur();
    }
    // Keys, unless the UI used them (a focused text input takes them all;
    // Tab/Enter go to a focused control).
    // Everything else with a key is a core binding (mods/core/ui/keys.luau).
    for k in raw.keys.iter().filter(|_| !out.captured_keys) {
        let action = match k {
            KeyCode::Tab => Action::NextColonist,
            _ => continue,
        };
        apply(app, action);
    }
    if raw.pan != (0.0, 0.0) && !out.captured_keys {
        apply(app, Action::Pan(raw.pan.0, raw.pan.1));
    }
    let (mx, my) = raw.mouse;
    // A pinch zooms around the pointer, which is where the fingers are, as
    // far as the fingers spread: continuous, so a slow pinch is slow. The UI
    // captures only the wheel, so a pinch over a panel is stopped here.
    if raw.pinch != 0.0 && !out.mouse_over_ui {
        apply(app, Action::Zoom((1.0 + raw.pinch).clamp(0.5, 2.0), mx, my));
    }
    if !out.captured_wheel {
        scroll_camera(app, raw);
    }
    if raw.left_pressed && !out.captured_left {
        apply(app, Action::LeftDown(mx, my));
    }
    if raw.left_released && app.drag_start.is_some() {
        apply(app, Action::LeftUp(mx, my));
    }
    right_button(app, raw, &cv, out.captured_right);
    // The camera moving takes the player's attention off an open menu.
    if (app.cam.x, app.cam.y, app.cam.zoom) != cam_before {
        app.ui.dismiss_popups(&app.sim.world, &cv);
    }

    // Orders show the frame they're given, paused or not.
    app.sim.apply_pending();
    if raw.advance {
        step(app);
    }
    prune_selection(app);
    hint(app, raw.mouse);
    let picked = selection(app);
    // A drag counts once it leaves the cell it started in.
    let dragging = app.drag_start.is_some_and(|a| a != app.cam.tile_at(mx, my));
    app.pointer = (mx, my);
    app.zone_preview = draw::ZonePreview::of(app);
    app.order_preview = overlay::OrderPreview::of(app);
    let last = app.build_preview.take();
    app.build_preview = overlay::BuildPreview::of(app, last);
    let pointer = (!app.mouse_over_ui).then(|| app.cam.to_world(mx, my));
    app.grid.update(grid::level(app.tool, dragging), app.measure, pointer, raw.time, app.reduce_motion);
    let hovered = if app.tool == Tool::Select && !dragging && pointer.is_some() { hovered(app, mx, my) } else { None };
    app.chalk.update(&picked, hovered, raw.time, app.reduce_motion);
}

fn step(app: &mut App) {
    if app.paused || app.sim.world.colony_lost {
        // Keep the part-tick: pawns are drawn that far along their step,
        // and dropping it would pull them back a little on pause.
        app.acc = app.acc.fract();
        // No game time passes, so nothing turns; a pawn seen for the first
        // time still gets a facing.
        let frac = app.tick_frac();
        app.motion.face(&app.sim.world, &app.worksites, frac);
        return;
    }
    app.acc += frame_time() as f64 * 60.0 * app.speed as f64;
    let budget = std::time::Instant::now();
    while app.acc >= 1.0 {
        app.sim.step();
        app.motion.stepped(&app.sim.world);
        if let Some(w) = &app.saver {
            save::after_step(w, &mut app.sim);
        }
        app.acc -= 1.0;
        if budget.elapsed().as_millis() > 12 {
            app.acc = app.acc.min(4.0);
            break;
        }
    }
    let frac = app.tick_frac();
    app.motion.face(&app.sim.world, &app.worksites, frac);
}

/// Drop from the selection what is no longer there. Orders land while
/// paused too (a cancelled plan, a cleared zone), so this runs every frame,
/// not only when time passed.
fn prune_selection(app: &mut App) {
    let w = &app.sim.world;
    // A thing someone picked up is in their hand, not on the map.
    let gone = |e| !w.pawn_alive(e) && (w.thing(e).is_none() || w.ecs.get::<&rim_sim::world::Held>(e).is_ok());
    if app.selected.is_some_and(gone) || app.group.iter().any(|&e| gone(e)) {
        let keep: Vec<Entity> = selection(app).into_iter().filter(|&e| !gone(e)).collect();
        select(app, keep);
    }
    if app.selected_zone.is_some_and(|z| app.sim.world.zones.get(z).is_none()) {
        app.selected_zone = None;
    }
}

impl App {
    /// How far into the next sim tick this frame falls: what the step
    /// accumulator holds past its last whole tick. Pawns are drawn that
    /// far along their step, so they glide at any frame rate.
    pub fn tick_frac(&self) -> f32 {
        (self.acc as f32).clamp(0.0, 0.999)
    }
}
