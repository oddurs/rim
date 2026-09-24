//! rim client: renders the simulation and turns input into Commands.
//! The client never mutates the world directly.
//!
//! Each frame: raw input goes to the UI first (rim_ui routes it against the
//! last layout; panels swallow clicks). UI actions apply, then whatever the
//! UI didn't take drives the world. The HUD itself is core's UI mod.

mod autotest;
mod draw;

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
/// scripts can name it ("designate:chop", "build:wall_wood").
pub struct ToolDef {
    pub key: String,
    pub label: String,
    pub tool: Tool,
    pub color: Color,
}

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
    /// Input subscriber for wheel events (see `Wheel`).
    wheel_sub: usize,
}

fn conf() -> Conf {
    Conf {
        window_title: "rim".to_owned(),
        window_width: 1600,
        window_height: 960,
        window_resizable: true,
        high_dpi: true,
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

#[macroquad::main(conf)]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed = args
        .windows(2)
        .find(|w| w[0] == "--seed")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    let ui_scale: f32 = args.windows(2).find(|w| w[0] == "--ui-scale").and_then(|w| w[1].parse().ok()).unwrap_or(1.0);

    let sim = match find_mods()
        .ok_or_else(|| "could not find a mods/ directory".to_string())
        .and_then(|d| Sim::new(&d, seed))
    {
        Ok(s) => s,
        Err(e) => return fail(e).await,
    };
    let ui = match Ui::new(rim_ui::mods_of(&sim), screen_dpi_scale(), ui_scale) {
        Ok(u) => u,
        Err(e) => return fail(format!("UI failed to start: {e}")).await,
    };
    eprintln!("rim: seed {seed}, {} mods loaded, UI font {}", sim.mods.len(), ui.info.font);
    for w in sim.warnings.iter().chain(&ui.warnings()) {
        eprintln!("  warning: {w}");
    }

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
        hint_key: None,
        order_flash: None,
        mouse_over_ui: false,
        last_draw: Vec::new(),
        profile: (Vec::new(), Vec::new(), f64::MIN),
        acc: 0.0,
        pan_anchor: None,
        wheel_sub: macroquad::input::utils::register_input_subscriber(),
    };
    app.selected = app.sim.world.colonists().next();

    if let Some(i) = args.iter().position(|a| a == "--autotest") {
        let dir = args.get(i + 1).filter(|a| !a.starts_with("--")).map_or("target/autotest".into(), PathBuf::from);
        autotest::run(app, dir).await;
    }

    loop {
        let raw = RawInput::gather(&mut app);
        frame(&mut app, &raw);
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
            KeyCode::Space,
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::F3,
            KeyCode::P,
            KeyCode::O,
            KeyCode::F12,
            KeyCode::Escape,
            KeyCode::R,
            KeyCode::Tab,
            KeyCode::C,
            KeyCode::Enter,
        ]
        .into_iter()
        .filter(|k| is_key_pressed(*k))
        .collect();

        let speed = 18.0 * get_frame_time() * 40.0 / app.cam.zoom;
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
        app.profile = (
            rows,
            vec![
                format!("tick {} · pawns {} · entities {}", w.tick, w.pawns.len(), w.ecs.len()),
                format!("paths {} · nodes {} · reservations {}", w.pf.searches, w.pf.expanded, w.reservations.len()),
                format!("script hooks {hooks} · handlers {handlers} · emitters {}", w.fields.emitter_count()),
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
        time: raw.time,
    };
    let out = app.ui.frame(&app.sim.world, &cv, &input);
    app.last_draw = out.draw;
    app.mouse_over_ui = out.mouse_over_ui;
    for a in out.actions {
        apply_ui(app, a);
    }

    // Keys, unless the UI used them (Tab/Enter while a UI control has focus).
    for k in &raw.keys {
        let action = match k {
            KeyCode::Space => Action::TogglePause,
            KeyCode::Key1 => Action::Speed(1),
            KeyCode::Key2 => Action::Speed(3),
            KeyCode::Key3 => Action::Speed(6),
            KeyCode::F3 | KeyCode::P => Action::ToggleProfiler,
            KeyCode::F12 => Action::ToggleDevtools,
            KeyCode::O => Action::CycleOverlay,
            KeyCode::Escape => {
                app.ui.blur();
                Action::Escape
            }
            KeyCode::R => Action::ToggleDraft,
            KeyCode::Tab if !out.captured_keys => Action::NextColonist,
            KeyCode::C => Action::CenterSelected,
            _ => continue,
        };
        apply(app, action);
    }
    if raw.pan != (0.0, 0.0) {
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

pub fn render(app: &mut App) {
    draw::world(app);
    if app.ui.text.atlas.dirty {
        let a = &app.ui.text.atlas;
        app.atlas.update(&Image { bytes: a.pixels.clone(), width: a.size as u16, height: a.size as u16 });
        app.ui.text.atlas.dirty = false;
    }
    draw::ui(&app.last_draw, &app.atlas, screen_dpi_scale());
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
        UiAction::Speed(s) => apply(app, Action::Speed(s)),
        UiAction::TogglePause => apply(app, Action::TogglePause),
        UiAction::Draft(e, on) => app.sim.push(Command::Draft { pawn: e, on }),
        UiAction::CycleOverlay => apply(app, Action::CycleOverlay),
        UiAction::SetOverlay(o) => app.overlay = o.filter(|i| *i < app.sim.world.defs.fields.len()),
        UiAction::ToggleProfiler => apply(app, Action::ToggleProfiler),
        UiAction::ToggleDevtools => apply(app, Action::ToggleDevtools),
        UiAction::ToggleOutlines => app.ui.toggle_outlines(),
    }
}

fn step(app: &mut App) {
    if app.paused || app.sim.world.colony_lost {
        app.acc = 0.0;
        return;
    }
    app.acc += get_frame_time() as f64 * 60.0 * app.speed as f64;
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
            let n = app.sim.world.defs.fields.len();
            app.overlay = match app.overlay {
                None if n > 0 => Some(0),
                Some(i) if i + 1 < n => Some(i + 1),
                _ => None,
            };
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
            app.cam.zoom = (app.cam.zoom * f).clamp(4.0, 80.0);
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
                    for (a, b) in build_rects(defs.thing(t).blocks, a, b) {
                        app.sim.push(Command::Build { thing: t, a, b });
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
