//! `rim --bench-render`: the renderer against its budget (DESIGN.md §8).
//!
//! The §8 world (250×250, 30 colonists, 200 pawns) with a colony stamped
//! in around the start: rooms of walls, doors, floors, furniture and
//! stacks, some walls still planned, and every designation over the rest
//! of the map. It is drawn through the game's own `frame` and `render` in
//! four views: the whole map at the lowest zoom, mid, close, and the whole
//! map in a storm. Per view: each pass's CPU time, the time macroquad
//! takes to hand the frame to GL ("submit"), the time the GPU takes to
//! finish it (Linux only, where macroquad calls glFinish under telemetry),
//! and one frame's draw calls and indices.
//!
//! `--check` exits 1 when the world's CPU time on the whole map is over
//! budget. `--json FILE` writes the numbers, `--shots DIR` saves a
//! screenshot of each view, and `--frames N` sets the frames per view.

use crate::{frame, render, App, RawInput, RenderTimes, MIN_ZOOM};
use macroquad::prelude::*;
use macroquad::telemetry;
use rim_sim::defs::Category;
use rim_sim::world::Faction;
use rim_sim::{Command, IVec, Sim};
use std::path::Path;

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

/// The §8 world with a colony in it.
pub fn world(mods: &Path, seed: u64) -> Result<Sim, String> {
    let mut s = Sim::build(mods, seed, &|_| true, SIZE)?;
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
                let _ = s.world.spawn_fixture_of(floor as _, p, false, stuff(floor, room));
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
                let _ = s.world.spawn_fixture_of(d as _, p, planned && d == wall, stuff(d, room));
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
}

const VIEWS: [View; 4] = [
    View { name: "whole map", zoom: None, storm: false },
    View { name: "mid", zoom: Some(12.0), storm: false },
    View { name: "close", zoom: Some(28.0), storm: false },
    View { name: "storm", zoom: None, storm: true },
];

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
        for _ in 0..30 {
            draw_one(&mut app, &mut time, None).await;
        }
        let mut r = Run { name: v.name, zoom: app.cam.zoom, ..Default::default() };
        for _ in 0..frames {
            draw_one(&mut app, &mut time, None).await;
            let zones = telemetry::frame().zones;
            let submit = zone(&zones, "Event::draw end_frame").unwrap_or(0.0);
            r.frames.push((app.render_us, submit, zone(&zones, "glFinish/glFLush")));
        }
        // The capture is taken on the frame after the one that asks.
        telemetry::capture_frame();
        draw_one(&mut app, &mut time, None).await;
        let shot = shots.as_ref().map(|d| d.join(format!("{}.png", v.name.replace(' ', "_"))));
        draw_one(&mut app, &mut time, shot.as_deref()).await;
        let calls = telemetry::drawcalls();
        r.calls = calls.len();
        r.indices = calls.iter().map(|c| c.indices_count).sum();
        r.particles = app.sky.particles();
        results.push(r);
    }

    let dpi = screen_dpi_scale();
    println!(
        "render bench: {SIZE}×{SIZE}, {} colonists, {} pawns, {}×{} points at {dpi}x, {frames} frames per view",
        app.sim.world.colonists().count(),
        app.sim.world.pawns.len(),
        screen_width(),
        screen_height()
    );
    println!(
        "{:<10} {:>5} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>6} {:>8}",
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
        "indices"
    );
    for r in &results {
        let sorted = r.world_ms();
        println!(
            "{:<10} {:>5.0} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7} {:>6} {:>8}",
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
            r.indices
        );
    }
    println!("ms per frame, CPU unless named; world = every pass but the UI; budget {BUDGET_MS} ms on the whole map");

    if let Some(path) = opt("--json") {
        let views: Vec<String> = results
            .iter()
            .map(|r| {
                let sorted = r.world_ms();
                format!(
                    "    {{\"view\": \"{}\", \"zoom\": {}, \"world_ms\": {:.4}, \"world_p50_ms\": {:.4}, \"world_p99_ms\": {:.4}, \"ground_ms\": {:.4}, \"things_ms\": {:.4}, \"pawns_ms\": {:.4}, \"weather_ms\": {:.4}, \"light_ms\": {:.4}, \"ui_ms\": {:.4}, \"submit_ms\": {:.4}, \"gpu_ms\": {}, \"draw_calls\": {}, \"indices\": {}, \"particles\": {}}}",
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
                    r.particles
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
        let slack = if std::env::var_os("CI").is_some() { 2.0 } else { 1.0 };
        let limit = BUDGET_MS * slack;
        let mean = results[0].mean(|f| f.0.world());
        if mean > limit {
            eprintln!("render bench: the whole map takes {mean:.3} ms of CPU, over the budget of {limit:.1} ms");
            std::process::exit(1);
        }
        println!("render bench: within budget ({mean:.3} <= {limit:.1} ms)");
    }
    std::process::exit(0)
}
