//! `rim --bench-render`: the renderer against its budget (DESIGN.md §8).
//!
//! The §8 world (250×250, 30 colonists, 200 pawns) with a colony stamped
//! in around the start: rooms of walls, doors, floors, furniture and
//! stacks, some walls still planned, and every designation over the rest
//! of the map. It is drawn through the game's own `frame` and `render` in
//! six views: the whole map at the lowest zoom, mid, close, the whole map
//! in a storm, a zoom gesture from the whole map to close and back, and the
//! storm again at half render scale. Per view: each pass's CPU time, the time macroquad
//! takes to hand the frame to GL ("submit"), the time the GPU takes to
//! finish it (Linux only, where macroquad calls glFinish under telemetry),
//! one frame's draw calls and indices, and how many things it draws live
//! rather than from the chunk meshes.
//!
//! `--check` exits 1 when the world's CPU time on the whole map (clear, in
//! a storm, or zooming through it) is over budget. `--sprite-mods N` adds N
//! generated mods whose furniture is drawn from sprites. `--json FILE`
//! writes the numbers, `--shots DIR` saves a screenshot of each view, and
//! `--frames N` sets the frames per view.

use crate::{frame, render, App, RawInput, RenderTimes, MIN_ZOOM};
use macroquad::prelude::*;
use macroquad::telemetry;
use rim_sim::defs::Category;
use rim_sim::world::{Faction, Owner};
use rim_sim::{Command, IVec, Sim};
use std::path::{Path, PathBuf};

/// The world renderer's CPU budget per frame on the reference machine, in
/// ms (DESIGN.md §8).
const BUDGET_MS: f64 = 4.0;
const SIZE: i32 = 250;
const COLONISTS: usize = 30;
const PAWNS: usize = 200;
/// Room pitch in the stamped colony: walls every `ROOM` cells.
const ROOM: i32 = 8;
/// Side of the stamped colony, in cells.
const COLONY: i32 = 96;

/// A copy of `mods` plus `n` generated mods, each a piece of furniture
/// drawn from a sprite of its own: the colony gets built of them, so the
/// bench shows what twenty mods' art costs in draw calls.
fn with_sprite_mods(mods: &Path, n: usize) -> Result<PathBuf, String> {
    fn copy(from: &Path, to: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(to)?;
        for e in std::fs::read_dir(from)?.flatten() {
            let p = e.path();
            if p.is_dir() {
                copy(&p, &to.join(e.file_name()))?;
            } else {
                std::fs::copy(&p, to.join(e.file_name()))?;
            }
        }
        Ok(())
    }
    let err = |e: std::io::Error| format!("render bench: sprite mods: {e}");
    // Fresh each run: a left-over folder from a reused pid would add mods.
    let dir = std::env::temp_dir().join(format!("rim-bench-sprites-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(err)?;
    }
    copy(mods, &dir).map_err(err)?;
    let art = mods.join("wildlife_plus/sprites/salt_lick.png");
    for k in 0..n {
        let m = dir.join(format!("art{k:02}"));
        std::fs::create_dir_all(m.join("sprites")).map_err(err)?;
        std::fs::create_dir_all(m.join("defs")).map_err(err)?;
        std::fs::copy(&art, m.join("sprites/piece.png")).map_err(err)?;
        let manifest = format!(
            "id = \"art{k:02}\"\nname = \"Art {k}\"\nversion = \"0.0.0\"\napi = \"0.5\"\ndepends = [\"core\"]\n"
        );
        std::fs::write(m.join("mod.toml"), manifest).map_err(err)?;
        let def = "[[thing]]\nid = \"piece\"\nlabel = \"piece\"\ncolor = \"#a08060\"\ncategory = \"building\"\n\
                   path_cost = 50\n\
                   build = { menu = \"furniture\", work = 10, stuff = { category = \"structural\", count = 1 } }\n\
                   look.layers = [{ draw = \"sprite\", sprite = \"piece\", tint = true }]\n";
        std::fs::write(m.join("defs/piece.toml"), def).map_err(err)?;
    }
    Ok(dir)
}

/// The §8 world with a colony in it, and `sprite_mods` generated art mods.
pub fn world(mods: &Path, seed: u64, sprite_mods: usize) -> Result<Sim, String> {
    let mods = if sprite_mods > 0 { with_sprite_mods(mods, sprite_mods)? } else { mods.to_path_buf() };
    let mut s = Sim::build(&mods, seed, &|_| true, SIZE)?;
    let defs = s.world.defs.clone();
    let c = s.world.colony_center().ok_or("render bench: the map has no colony")?;
    let mut rng = rim_sim::rng::Rng::new(seed ^ 0xBE7C);

    let start = defs.start.as_ref().ok_or("render bench: no start")?.creature_r;
    let mut tries = 0;
    while s.world.colonists().count() < COLONISTS && tries < 10_000 {
        tries += 1;
        let p = c.offset(rng.range(-12, 12), rng.range(-12, 12));
        if s.world.map.passable(p) {
            s.world.spawn_pawn(start, Faction::Player, p, None);
        }
    }
    let wild: Vec<_> = (0..defs.creatures.len()).filter(|&d| d != start as usize).collect();
    tries = 0;
    while !wild.is_empty() && s.world.pawns.len() < PAWNS && tries < 100_000 {
        tries += 1;
        let p = IVec::new(rng.range(0, SIZE - 1), rng.range(0, SIZE - 1));
        if s.world.map.passable(p) {
            let def = wild[rng.below(wild.len() as u32) as usize];
            s.world.spawn_pawn(def as _, Faction::Wild, p, None);
        }
    }

    // What to build it from, chosen by what defs are rather than by name.
    let buildable = |d: usize| defs.things[d].build.is_some();
    let of =
        |pred: &dyn Fn(usize) -> bool| (0..defs.things.len()).filter(|&d| buildable(d) && pred(d)).collect::<Vec<_>>();
    let walls = of(&|d| defs.things[d].blocks && !defs.things[d].door);
    let doors = of(&|d| defs.things[d].door);
    let floors = of(&|d| defs.things[d].category == Category::Floor);
    let furniture = of(&|d| {
        let t = &defs.things[d];
        t.category == Category::Building && !t.blocks && !t.door
    });
    let items: Vec<_> = (0..defs.things.len()).filter(|&d| defs.things[d].category == Category::Item).collect();
    let (Some(&wall), Some(&door), Some(&floor)) = (walls.first(), doors.first(), floors.first()) else {
        return Err("render bench: the mods have no wall, door or floor to build".into());
    };
    let stuff = |d: usize, k: usize| {
        let sc = defs.things[d].build.as_ref()?.stuff.as_ref()?;
        let m = defs.materials(&sc.category);
        (!m.is_empty()).then(|| m[k % m.len()])
    };

    // The colony built it, so it owns it: deconstruct marks only what is ours.
    let built = |s: &mut Sim, e: Option<rim_sim::hecs::Entity>| {
        if let Some(e) = e {
            let _ = s.world.ecs.insert_one(e, Owner(Faction::Player));
        }
    };
    let o = c.offset(-COLONY / 2, -COLONY / 2);
    for y in 0..=COLONY {
        for x in 0..=COLONY {
            let p = o.offset(x, y);
            if !s.world.map.inb(p) {
                continue;
            }
            for e in [s.world.map.fixture_at(p), s.world.map.item_at(p), s.world.map.floor_at(p)].into_iter().flatten()
            {
                s.world.despawn_thing(e);
            }
            let room = ((y / ROOM) * (COLONY / ROOM + 1) + x / ROOM) as usize;
            let (lx, ly) = (x % ROOM, y % ROOM);
            // One room in six is still being built.
            let planned = room % 6 == 5;
            let fixture = if ly == 0 && lx == ROOM / 2 {
                Some(door)
            } else if lx == 0 || ly == 0 {
                Some(wall)
            } else {
                let e = s.world.spawn_fixture_of(floor as _, p, false, stuff(floor, room));
                built(&mut s, e);
                match (lx, ly) {
                    (2, 2) | (5, 2) | (2, 5) if !furniture.is_empty() => {
                        Some(furniture[(room + lx as usize) % furniture.len()])
                    }
                    _ => {
                        if ly >= 4 && lx >= 4 && !items.is_empty() {
                            let d = items[(room + lx as usize) % items.len()];
                            s.world.place_item(d as _, p, (defs.things[d].stack_limit / 2).max(1));
                        }
                        None
                    }
                }
            };
            if let Some(d) = fixture {
                let e = s.world.spawn_fixture_of(d as _, p, planned && d == wall, stuff(d, room));
                if !(planned && d == wall) {
                    built(&mut s, e);
                }
            }
        }
    }

    // Every designation over the whole map: markers wherever they apply.
    for d in 0..defs.designations.len() {
        s.push(Command::Designate { designation: d as _, a: IVec::new(0, 0), b: IVec::new(SIZE - 1, SIZE - 1) });
    }
    for _ in 0..30 {
        s.step();
    }
    Ok(s)
}

struct View {
    name: &'static str,
    /// None: the lowest zoom, centred on the map.
    zoom: Option<f32>,
    storm: bool,
    /// Zoom in and out through the measured frames, as a player does.
    zooming: bool,
    /// The world's resolution (render scale).
    scale: f32,
}

const VIEWS: [View; 6] = [
    View { name: "whole map", zoom: None, storm: false, zooming: false, scale: 1.0 },
    View { name: "mid", zoom: Some(12.0), storm: false, zooming: false, scale: 1.0 },
    View { name: "close", zoom: Some(28.0), storm: false, zooming: false, scale: 1.0 },
    View { name: "storm", zoom: None, storm: true, zooming: false, scale: 1.0 },
    View { name: "zooming", zoom: Some(12.0), storm: false, zooming: true, scale: 1.0 },
    // The storm at half the pixels: what render scale saves the GPU.
    View { name: "storm 50%", zoom: None, storm: true, zooming: false, scale: 0.5 },
];

/// Frames in one zoom gesture, lowest zoom to close and back.
const GESTURE: usize = 120;

/// The zoom `k` frames into a gesture: geometric, like the wheel.
fn gesture_zoom(k: usize) -> f32 {
    let t = (k % GESTURE) as f32 / GESTURE as f32;
    let tri = 1.0 - (2.0 * t - 1.0).abs();
    MIN_ZOOM * (28.0 / MIN_ZOOM).powf(tri)
}

/// The weather channels the renderer reads, pinned: clear, or a storm.
const CHANNELS: [&str; 6] = ["precipitation", "temperature", "wind", "wind_dir", "cloud", "fog"];
const CLEAR: [f64; 6] = [0.0, 15.0, 2.0, 0.0, 20.0, 0.0];
const STORM: [f64; 6] = [8.0, 9.0, 16.0, 20.0, 100.0, 30.0];

#[derive(Default)]
struct Run {
    name: &'static str,
    zoom: f32,
    /// Per frame: the render passes, submit and GPU time (µs).
    frames: Vec<(RenderTimes, f64, Option<f64>)>,
    calls: usize,
    indices: usize,
    particles: usize,
    /// Things drawn live, outside the chunk meshes, on the last frame.
    live: usize,
    /// Chunk meshes rebuilt over the measured frames.
    rebuilt: usize,
    /// Held to the budget: the whole map, and a zoom gesture through it.
    gated: bool,
}

impl Run {
    fn world_ms(&self) -> Vec<f64> {
        let mut v: Vec<f64> = self.frames.iter().map(|f| f.0.world() / 1e3).collect();
        v.sort_by(f64::total_cmp);
        v
    }

    fn mean(&self, f: impl Fn(&(RenderTimes, f64, Option<f64>)) -> f64) -> f64 {
        self.frames.iter().map(f).sum::<f64>() / self.frames.len().max(1) as f64 / 1e3
    }

    fn gpu_ms(&self) -> Option<f64> {
        let v: Vec<f64> = self.frames.iter().filter_map(|f| f.2).collect();
        (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64 / 1e3)
    }
}

fn pct(sorted: &[f64], p: f64) -> f64 {
    sorted.get(((sorted.len() as f64 * p) as usize).min(sorted.len().saturating_sub(1))).copied().unwrap_or(0.0)
}

/// A zone's duration in the last frame, in µs, searched by name.
fn zone(zones: &[telemetry::Zone], name: &str) -> Option<f64> {
    zones.iter().find_map(|z| if z.name == name { Some(z.duration * 1e6) } else { zone(&z.children, name) })
}

fn pin(app: &mut App, values: [f64; 6]) {
    for (id, v) in CHANNELS.iter().zip(values) {
        if let Some(f) = app.sim.world.defs.lookup("field", id) {
            app.sim.world.fields.set_ambient(f as usize, Some(v));
        }
    }
}

/// Draw a frame. With `shot`, read it back first: after `next_frame` the
/// buffer has been swapped away and reads black.
async fn draw_one(app: &mut App, time: &mut f64, shot: Option<&Path>) {
    *time += 1.0 / 60.0;
    frame(app, &RawInput { time: *time, ..Default::default() });
    render(app);
    if let Some(p) = shot {
        get_screen_data().export_png(&p.to_string_lossy());
    }
    next_frame().await;
}

pub async fn run(mut app: App, args: &[String]) -> ! {
    let opt = |name: &str| args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone());
    let frames: usize = opt("--frames").and_then(|v| v.parse().ok()).unwrap_or(300);
    let check = args.iter().any(|a| a == "--check");
    let shots = opt("--shots").map(std::path::PathBuf::from);
    if let Some(d) = &shots {
        if let Err(e) = std::fs::create_dir_all(d) {
            eprintln!("render bench: could not create {}: {e}", d.display());
            std::process::exit(2);
        }
    }
    let centre = app.sim.world.colony_center().unwrap_or(IVec::new(SIZE / 2, SIZE / 2));
    app.selected = None;
    telemetry::enable();
    // The reference screen, in points: the whole map fits at the lowest
    // zoom. The request is in pixels on a high-DPI screen.
    let dpi = screen_dpi_scale();
    request_new_screen_size(1920.0 / dpi, 1080.0 / dpi);
    next_frame().await;
    let mut time = 0.0;

    let mut results = Vec::new();
    for v in &VIEWS {
        pin(&mut app, if v.storm { STORM } else { CLEAR });
        // Pinned channels reach the world's outdoor values on a tick.
        app.sim.step();
        let (w, h) = (app.sim.world.map.w as f32, app.sim.world.map.h as f32);
        let (x, y) = match v.zoom {
            None => (w / 2.0, h / 2.0),
            Some(_) => (centre.x as f32 + 0.5, centre.y as f32 + 0.5),
        };
        app.cam.x = x;
        app.cam.y = y;
        app.cam.zoom = v.zoom.unwrap_or(MIN_ZOOM);
        app.render_scale = Some(v.scale);
        // A gesture warms up on the gesture, so measuring doesn't start
        // with a jump from wherever the last view left the zoom.
        const WARM: usize = 30;
        for k in 0..WARM {
            if v.zooming {
                app.cam.zoom = gesture_zoom(k);
            }
            draw_one(&mut app, &mut time, None).await;
        }
        let mut r =
            Run { name: v.name, zoom: app.cam.zoom, gated: v.zoom.is_none() || v.zooming, ..Default::default() };
        for k in WARM..WARM + frames {
            if v.zooming {
                app.cam.zoom = gesture_zoom(k);
            }
            draw_one(&mut app, &mut time, None).await;
            r.rebuilt += app.meshes.rebuilt;
            let zones = telemetry::frame().zones;
            // Submit: macroquad's end of frame, and the meshes' mid-frame.
            let submit = zone(&zones, "Event::draw end_frame").unwrap_or(0.0) + app.render_us.gl;
            r.frames.push((app.render_us, submit, zone(&zones, "glFinish/glFLush")));
        }
        // A gesture's numbers are for the zoom it ended on.
        r.zoom = app.cam.zoom;
        // The capture is taken on the frame after the one that asks.
        telemetry::capture_frame();
        draw_one(&mut app, &mut time, None).await;
        let shot = shots.as_ref().map(|d| d.join(format!("{}.png", v.name.replace(' ', "_").replace('%', ""))));
        draw_one(&mut app, &mut time, shot.as_deref()).await;
        // Macroquad's capture sees its own batches; the chunk meshes are
        // drawn past it and count themselves.
        let calls = telemetry::drawcalls();
        r.calls = calls.len() + app.meshes.calls;
        r.indices = calls.iter().map(|c| c.indices_count).sum::<usize>() + app.meshes.indices;
        r.particles = app.sky.particles();
        r.live = app.meshes.live_count();
        results.push(r);
    }

    println!(
        "render bench: {SIZE}×{SIZE}, {} colonists, {} pawns, {}×{} points at {dpi}x, {frames} frames per view",
        app.sim.world.colonists().count(),
        app.sim.world.pawns.len(),
        screen_width(),
        screen_height()
    );
    println!(
        "{:<10} {:>5} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>6} {:>8} {:>7} {:>6}",
        "view",
        "zoom",
        "world",
        "p99",
        "ground",
        "things",
        "pawns",
        "weather",
        "light",
        "ui",
        "submit",
        "gpu",
        "calls",
        "indices",
        "rebuilt",
        "live"
    );
    for r in &results {
        let sorted = r.world_ms();
        println!(
            "{:<10} {:>5.0} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7} {:>6} {:>8} {:>7} {:>6}",
            r.name,
            r.zoom,
            r.mean(|f| f.0.world()),
            pct(&sorted, 0.99),
            r.mean(|f| f.0.ground),
            r.mean(|f| f.0.things),
            r.mean(|f| f.0.pawns),
            r.mean(|f| f.0.weather),
            r.mean(|f| f.0.light),
            r.mean(|f| f.0.ui),
            r.mean(|f| f.1),
            r.gpu_ms().map_or("-".into(), |g| format!("{g:.3}")),
            r.calls,
            r.indices,
            r.rebuilt,
            r.live
        );
    }
    println!("ms per frame, CPU unless named; world = every pass but the UI; budget {BUDGET_MS} ms on the whole map");

    if let Some(path) = opt("--json") {
        let views: Vec<String> = results
            .iter()
            .map(|r| {
                let sorted = r.world_ms();
                format!(
                    "    {{\"view\": \"{}\", \"zoom\": {}, \"world_ms\": {:.4}, \"world_p50_ms\": {:.4}, \"world_p99_ms\": {:.4}, \"ground_ms\": {:.4}, \"things_ms\": {:.4}, \"pawns_ms\": {:.4}, \"weather_ms\": {:.4}, \"light_ms\": {:.4}, \"ui_ms\": {:.4}, \"submit_ms\": {:.4}, \"gpu_ms\": {}, \"draw_calls\": {}, \"indices\": {}, \"particles\": {}, \"rebuilt\": {}, \"live\": {}}}",
                    r.name,
                    r.zoom,
                    r.mean(|f| f.0.world()),
                    pct(&sorted, 0.5),
                    pct(&sorted, 0.99),
                    r.mean(|f| f.0.ground),
                    r.mean(|f| f.0.things),
                    r.mean(|f| f.0.pawns),
                    r.mean(|f| f.0.weather),
                    r.mean(|f| f.0.light),
                    r.mean(|f| f.0.ui),
                    r.mean(|f| f.1),
                    r.gpu_ms().map_or("null".into(), |g| format!("{g:.4}")),
                    r.calls,
                    r.indices,
                    r.particles,
                    r.rebuilt,
                    r.live
                )
            })
            .collect();
        let json = format!(
            "{{\n  \"screen\": [{}, {}],\n  \"dpi\": {dpi},\n  \"frames\": {frames},\n  \"budget_ms\": {BUDGET_MS},\n  \"views\": [\n{}\n  ]\n}}\n",
            screen_width(),
            screen_height(),
            views.join(",\n")
        );
        if let Err(e) = std::fs::write(&path, json) {
            eprintln!("render bench: could not write {path}: {e}");
            std::process::exit(2);
        }
    }

    if check {
        // Shared CI runners are noisy and draw in software; the slack is
        // for that, not for the renderer.
        let slack = if std::env::var_os("CI").is_some() { 1.5 } else { 1.0 };
        let limit = BUDGET_MS * slack;
        // The worst of the gated views.
        let (name, mean) = results
            .iter()
            .filter(|r| r.gated)
            .map(|r| (r.name, r.mean(|f| f.0.world())))
            .fold(("", 0.0), |a, b| if b.1 > a.1 { b } else { a });
        if mean > limit {
            eprintln!("render bench: {name} takes {mean:.3} ms of CPU, over the budget of {limit:.1} ms");
            std::process::exit(1);
        }
        println!("render bench: within budget ({name}: {mean:.3} <= {limit:.1} ms)");
    }
    std::process::exit(0)
}
