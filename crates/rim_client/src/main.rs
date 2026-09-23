//! rim client: renders the simulation and turns input into Commands.
//! The client never mutates the world directly.

mod draw;

use macroquad::prelude::*;
use rim_sim::defs::{DefId, Targets};
use rim_sim::hecs::Entity;
use rim_sim::world::{Faction, Pawn};
use rim_sim::{Command, IVec, Sim};
use std::path::PathBuf;

pub const TOOLBAR_H: f32 = 44.0;
pub const TOPBAR_H: f32 = 28.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    Select,
    Designate(DefId),
    Build(DefId),
    Cancel,
}

pub struct Button {
    pub rect: Rect,
    pub label: String,
    pub tool: Tool,
    pub color: Color,
}

pub struct Cam {
    /// Center of the view, in tiles.
    pub x: f32,
    pub y: f32,
    /// Pixels per tile.
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
    pub cam: Cam,
    pub tool: Tool,
    pub selected: Option<Entity>,
    pub drag_start: Option<IVec>,
    pub paused: bool,
    pub speed: u32,
    pub show_profiler: bool,
    pub buttons: Vec<Button>,
    acc: f64,
    pan_anchor: Option<(f32, f32, f32, f32)>,
}

fn conf() -> Conf {
    Conf {
        window_title: "rim".to_owned(),
        window_width: 1600,
        window_height: 960,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    let seed = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .find(|w| w[0] == "--seed")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());

    let sim = match find_mods()
        .ok_or_else(|| "could not find a mods/ directory".to_string())
        .and_then(|d| Sim::new(&d, seed))
    {
        Ok(s) => s,
        Err(e) => loop {
            clear_background(Color::from_rgba(30, 20, 20, 255));
            draw_text("rim failed to start", 40.0, 60.0, 36.0, WHITE);
            for (i, line) in e.lines().enumerate() {
                draw_text(line, 40.0, 110.0 + i as f32 * 24.0, 22.0, Color::from_rgba(255, 180, 160, 255));
            }
            next_frame().await
        },
    };
    eprintln!("rim: seed {seed}, {} mods loaded", sim.mods.len());
    for w in &sim.warnings {
        eprintln!("  warning: {w}");
    }

    let center = sim.world.colony_center().unwrap_or(IVec::new(100, 100));
    let mut app = App {
        buttons: toolbar(&sim),
        sim,
        cam: Cam { x: center.x as f32 + 0.5, y: center.y as f32 + 0.5, zoom: 28.0 },
        tool: Tool::Select,
        selected: None,
        drag_start: None,
        paused: false,
        speed: 1,
        show_profiler: false,
        acc: 0.0,
        pan_anchor: None,
    };
    app.selected = app.sim.world.colonists().next();

    loop {
        input(&mut app);
        step(&mut app);
        draw::world(&app);
        draw::hud(&app);
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
fn toolbar(sim: &Sim) -> Vec<Button> {
    let defs = &sim.world.defs;
    let mut items: Vec<(String, Tool, Color)> = vec![("Select".into(), Tool::Select, GRAY)];
    for (i, d) in defs.designations.iter().enumerate() {
        items.push((d.label.clone(), Tool::Designate(i as DefId), rgb(d.rgb)));
    }
    let mut builds: Vec<(usize, &rim_sim::defs::ThingDef)> =
        defs.things.iter().enumerate().filter(|(_, t)| t.build.is_some()).collect();
    builds.sort_by_key(|(i, t)| (t.build.as_ref().unwrap().menu.clone(), *i));
    for (i, t) in builds {
        items.push((t.label.clone(), Tool::Build(i as DefId), rgb(t.rgb)));
    }
    items.push(("Cancel".into(), Tool::Cancel, Color::from_rgba(200, 80, 80, 255)));

    let mut x = 8.0;
    items
        .into_iter()
        .map(|(label, tool, color)| {
            let w = measure_text(&label, None, 18, 1.0).width + 22.0;
            let b = Button { rect: Rect::new(x, 0.0, w, TOOLBAR_H - 12.0), label, tool, color };
            x += w + 6.0;
            b
        })
        .collect()
}

pub fn rgb(c: [u8; 3]) -> Color {
    Color::from_rgba(c[0], c[1], c[2], 255)
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

fn pawn_under(app: &App, sx: f32, sy: f32) -> Option<Entity> {
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

fn toolbar_y() -> f32 {
    screen_height() - TOOLBAR_H + 6.0
}

fn input(app: &mut App) {
    let (mx, my) = mouse_position();
    let over_ui = my > screen_height() - TOOLBAR_H || my < TOPBAR_H;

    // Keyboard.
    if is_key_pressed(KeyCode::Space) {
        app.paused = !app.paused;
    }
    for (k, s) in [(KeyCode::Key1, 1), (KeyCode::Key2, 3), (KeyCode::Key3, 6)] {
        if is_key_pressed(k) {
            app.speed = s;
            app.paused = false;
        }
    }
    if is_key_pressed(KeyCode::F3) || is_key_pressed(KeyCode::P) {
        app.show_profiler = !app.show_profiler;
    }
    if is_key_pressed(KeyCode::Escape) {
        if app.tool != Tool::Select {
            app.tool = Tool::Select;
        } else {
            app.selected = None;
        }
        app.drag_start = None;
    }
    if is_key_pressed(KeyCode::R) {
        if let Some(e) = app.selected {
            let drafted = app.sim.world.ecs.get::<&Pawn>(e).map(|p| (p.faction == Faction::Player, p.drafted)).ok();
            if let Some((true, d)) = drafted {
                app.sim.push(Command::Draft { pawn: e, on: !d });
            }
        }
    }
    if is_key_pressed(KeyCode::Tab) {
        let cols: Vec<Entity> = app.sim.world.colonists().collect();
        if !cols.is_empty() {
            let i = app.selected.and_then(|s| cols.iter().position(|&c| c == s)).map_or(0, |i| (i + 1) % cols.len());
            app.selected = Some(cols[i]);
            focus(app, cols[i]);
        }
    }
    if is_key_pressed(KeyCode::C) {
        if let Some(e) = app.selected {
            focus(app, e);
        }
    }

    // Camera.
    let pan = 18.0 * get_frame_time() * 40.0 / app.cam.zoom;
    if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
        app.cam.y -= pan;
    }
    if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
        app.cam.y += pan;
    }
    if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
        app.cam.x -= pan;
    }
    if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
        app.cam.x += pan;
    }
    let (_, wheel) = mouse_wheel();
    if wheel != 0.0 && !over_ui {
        let before = app.cam.to_world(mx, my);
        app.cam.zoom = (app.cam.zoom * if wheel > 0.0 { 1.12 } else { 1.0 / 1.12 }).clamp(4.0, 80.0);
        let after = app.cam.to_world(mx, my);
        app.cam.x += before.0 - after.0;
        app.cam.y += before.1 - after.1;
    }
    if is_mouse_button_pressed(MouseButton::Middle) {
        app.pan_anchor = Some((mx, my, app.cam.x, app.cam.y));
    }
    if let Some((ax, ay, cx, cy)) = app.pan_anchor {
        if is_mouse_button_down(MouseButton::Middle) {
            app.cam.x = cx - (mx - ax) / app.cam.zoom;
            app.cam.y = cy - (my - ay) / app.cam.zoom;
        } else {
            app.pan_anchor = None;
        }
    }
    let (mw, mh) = (app.sim.world.map.w as f32, app.sim.world.map.h as f32);
    app.cam.x = app.cam.x.clamp(0.0, mw);
    app.cam.y = app.cam.y.clamp(0.0, mh);

    // Toolbar.
    if is_mouse_button_pressed(MouseButton::Left) && my > screen_height() - TOOLBAR_H {
        for b in &app.buttons {
            let r = Rect::new(b.rect.x, toolbar_y(), b.rect.w, b.rect.h);
            if r.contains(vec2(mx, my)) {
                app.tool = b.tool;
            }
        }
        return;
    }
    // Colonist bar.
    if is_mouse_button_pressed(MouseButton::Left) && my < TOPBAR_H {
        if let Some(e) = draw::colonist_bar_hit(app, mx, my) {
            app.selected = Some(e);
            focus(app, e);
        }
        return;
    }

    // World.
    let tile = app.cam.tile_at(mx, my);
    if is_mouse_button_pressed(MouseButton::Left) && !over_ui {
        match app.tool {
            Tool::Select => app.selected = pawn_under(app, mx, my),
            _ => app.drag_start = Some(tile),
        }
    }
    if is_mouse_button_released(MouseButton::Left) {
        if let Some(a) = app.drag_start.take() {
            let b = tile;
            match app.tool {
                Tool::Designate(d) => {
                    // Creature designations use a generous rect so clicks catch moving targets.
                    let (a, b) = if app.sim.world.defs.designations[d as usize].targets == Targets::Creature && a == b {
                        (a.offset(-1, -1), b.offset(1, 1))
                    } else {
                        (a, b)
                    };
                    app.sim.push(Command::Designate { designation: d, a, b });
                }
                Tool::Build(t) => app.sim.push(Command::Build { thing: t, a, b }),
                Tool::Cancel => app.sim.push(Command::Cancel { a, b }),
                Tool::Select => {}
            }
        }
    }
    if is_mouse_button_pressed(MouseButton::Right) && !over_ui {
        if app.tool != Tool::Select {
            app.tool = Tool::Select;
            app.drag_start = None;
            return;
        }
        let Some(e) = app.selected else { return };
        let drafted = app.sim.world.ecs.get::<&Pawn>(e).is_ok_and(|p| p.drafted);
        if !drafted {
            return;
        }
        match pawn_under(app, mx, my).filter(|&t| t != e) {
            Some(t) if app.sim.world.ecs.get::<&Pawn>(t).is_ok_and(|p| p.faction != Faction::Player) => {
                app.sim.push(Command::Attack { pawn: e, target: t })
            }
            _ => app.sim.push(Command::Move { pawn: e, to: tile }),
        }
    }
}

fn focus(app: &mut App, e: Entity) {
    if let Ok(p) = app.sim.world.ecs.get::<&Pawn>(e) {
        app.cam.x = p.pos.x as f32 + 0.5;
        app.cam.y = p.pos.y as f32 + 0.5;
    }
}
