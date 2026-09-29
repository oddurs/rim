//! `rim --bench-render`: the renderer against its budget (DESIGN.md §8).
//!
//! The §8 world (250×250, 30 colonists, 200 pawns) with a colony stamped
//! in around the start: rooms of walls, doors, floors, furniture and
//! stacks, some walls still planned, and every designation over the rest
//! of the map. It is drawn through the game's own `frame` and `render` in
//! nine views: the whole map at the lowest zoom, mid, close, the whole map
//! in a storm, a zoom gesture from the whole map to close and back, the
//! storm again at half render scale, a level dug out below the colony seen
//! from the surface through pits and then from the level itself, flooded
//! from a river in its wall, and the
//! whole map at dusk, when the colony's fires matter. Per view: each pass's CPU time, the time macroquad
//! takes to hand the frame to GL ("submit"), the time the GPU takes to
//! finish it (Linux only, where macroquad calls glFinish under telemetry),
//! one frame's draw calls and indices, and how many things it draws live
//! rather than from the chunk meshes. Then each lighting pass on its own:
//! CPU, and the GPU's time for it from a timer query, taken over extra
//! frames (DESIGN.md §6e): reading a query back waits for the GPU, which
//! would disturb the main numbers. On a tile-based GPU (Apple silicon) a
//! query around part of a frame times the tile pass it lands in, so read
//! GPU numbers from an immediate-mode GPU. Last, what a change costs: every
//! cached lighting result rebuilt, frame after frame, and the colony's chunk
//! painted again and its region of chunk meshes joined, as a new wall does.
//!
//! Then the whole frame per view: its wall time from one frame's start to
//! the next (median, p99 and worst), hitches (frames over twice the
//! median), and the rest no pass accounts for (the present, and waits). The
//! header names the machine, since CI's runner classes differ up to 3x.
//!
//! Then the pawns on their own (DESIGN.md §6h): every pawn, topped up to
//! 200, gathered where all of them are on screen, and drawn as a dot, a
//! silhouette and in full, with the pawns pass's CPU time, the figures and
//! parts drawn, and the batch's draw calls.
//!
//! `--sync` finishes the GPU's work before each present, so a frame's wall
//! time is its own cost, not a queue draining (where GL has no timer, as on
//! a Mac, that is the only way to see a GPU-bound frame for what it is).
//!
//! `--check` exits 1 when a view breaks budgets.toml's `render.gpu` caps,
//! or `render.software` under software GL: world CPU on the whole map
//! (clear, in a storm, or zooming through it), the whole frame's p99 and
//! worst, hitches, draw calls, and what a new wall's mesh costs; or when
//! 200 figures in full take over their budget, or the figures take more
//! than one call. `--sprite-mods N` adds N
//! generated mods whose furniture is drawn from sprites. `--json FILE`
//! writes the numbers, `--shots DIR` saves a screenshot of each view, and
//! `--frames N` sets the frames per view.

use crate::{frame, light::PassTime, render, App, RawInput, RenderTimes, MIN_ZOOM};
use macroquad::prelude::*;
use macroquad::telemetry;
use rim_sim::defs::Category;
use rim_sim::world::{Faction, Owner, Pawn};
use rim_sim::{Command, IVec, Sim, TICKS_PER_DAY};
use std::path::{Path, PathBuf};

/// The pawns pass's CPU budget per frame, in ms: 200 figures in full
/// (DESIGN.md §6h).
const FIGURES_MS: f64 = 0.4;
/// Points a cell the crowd is drawn at: a dot, a silhouette, in full.
const CROWD_ZOOMS: [f32; 3] = [7.0, 15.0, 40.0];
/// The crowd's grid: columns, and cells between neighbours. 20 by 10 at
/// two cells apart fits the reference screen at 40 points a cell.
const CROWD_COLUMNS: i32 = 20;
const CROWD_PITCH: i32 = 2;
const SIZE: i32 = 250;
const COLONISTS: usize = 30;
const PAWNS: usize = 200;
/// Room pitch in the stamped colony: walls every `ROOM` cells.
const ROOM: i32 = 8;
/// Side of the stamped colony, in cells.
const COLONY: i32 = 96;
/// Side of the room the stacked views dig below it.
const STACKED: i32 = 40;

/// A copy of `mods` plus `n` generated mods, each a piece of furniture
/// drawn from a sprite of its own: the colony gets built of them, so the
/// bench shows what twenty mods' art costs in draw calls.
/// A folder copied, with everything in it: a mods folder to add to.
pub(crate) fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)?.flatten() {
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()))?;
        } else {
            std::fs::copy(&p, to.join(e.file_name()))?;
        }
    }
    Ok(())
}

fn with_sprite_mods(mods: &Path, n: usize) -> Result<PathBuf, String> {
    let err = |e: std::io::Error| format!("render bench: sprite mods: {e}");
    // Fresh each run: a left-over folder from a reused pid would add mods.
    let dir = std::env::temp_dir().join(format!("rim-bench-sprites-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(err)?;
    }
    copy_dir(mods, &dir).map_err(err)?;
    let art = mods.join("wildlife_plus/sprites/salt_lick.png");
    for k in 0..n {
        let m = dir.join(format!("art{k:02}"));
        std::fs::create_dir_all(m.join("sprites")).map_err(err)?;
        std::fs::create_dir_all(m.join("defs")).map_err(err)?;
        std::fs::copy(&art, m.join("sprites/piece.png")).map_err(err)?;
        let manifest = format!(
            "id = \"art{k:02}\"\nname = \"Art {k}\"\nversion = \"0.0.0\"\napi = \"0.8\"\ndepends = [\"core\"]\n"
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

/// The stacked scene (DESIGN.md §6d): a room `size` cells square dug out
/// of the level below `top_left`, a way down to it, three pits over it
/// and three colonists in it. Chosen by what defs are, not by name: the
/// first portal that isn't a hole, the first air terrain. Returns the
/// top of the way down, or None when the mods have neither.
pub fn stacked(s: &mut Sim, top_left: IVec, size: i32) -> Option<IVec> {
    let defs = s.world.defs.clone();
    let way = (0..defs.things.len()).find(|&d| {
        let t = &defs.things[d];
        t.portal.is_some() && t.build.as_ref().and_then(|b| b.dig.as_ref()).is_some_and(|g| g.hole.is_none())
    })? as rim_sim::defs::DefId;
    let air = defs.terrain.iter().position(|t| t.air)? as rim_sim::defs::DefId;
    let start = defs.start.as_ref()?.creature_r;
    let below = |p: IVec| IVec::at(p.x, p.y, p.z - 1);
    for y in 0..size {
        for x in 0..size {
            let p = below(top_left.offset(x, y));
            if let Some(leaves) = s.world.solid_at(p).and_then(|r| r.leaves_r) {
                s.world.map.set_terrain(p, leaves, defs.terrain[leaves as usize].path_cost);
            }
        }
    }
    let clear = |s: &Sim, p: IVec| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none();
    let top = (1..size - 1)
        .flat_map(|y| (1..size - 1).map(move |x| top_left.offset(x, y)))
        .find(|&p| clear(s, p) && s.world.map.passable(below(p)))?;
    let e = s.world.spawn_fixture_of(way, top, false, None)?;
    let _ = s.world.ecs.insert_one(e, Owner(Faction::Player));
    s.world.open_portal(e);
    // Pits: open ground over the room goes to air, whatever stood on it
    // with it.
    for k in 1..=3 {
        let o = top_left.offset(size * k / 4 - 1, size * k / 4 - 1);
        for p in (0..3).flat_map(|y| (0..3).map(move |x| o.offset(x, y))) {
            if p == top || !s.world.map.passable(p) || !s.world.map.passable(below(p)) {
                continue;
            }
            for e in [s.world.map.fixture_at(p), s.world.map.item_at(p), s.world.map.floor_at(p)].into_iter().flatten()
            {
                s.world.despawn_thing(e);
            }
            s.world.map.set_terrain(p, air, 0);
        }
    }
    for k in 0..3 {
        let p = below(top_left.offset(size / 2 + k, size / 2));
        if s.world.map.passable(p) {
            s.world.spawn_pawn(start, Faction::Player, p, None);
        }
    }
    Some(top)
}

/// A light in one room in four of the stamped colony, about forty: the
/// fires a dusk colony is lit by. Only the dusk view has them, so the
/// other views stay comparable with every earlier run.
fn light_the_colony(s: &mut Sim) {
    let defs = s.world.defs.clone();
    // Whatever gives light, chosen by what defs do rather than by name.
    let lamps: Vec<usize> = (0..defs.things.len())
        .filter(|&d| defs.things[d].build.is_some() && defs.things[d].emit.iter().any(|e| e.field == "light"))
        .collect();
    if lamps.is_empty() {
        return;
    }
    let Some(c) = s.world.colony_center() else { return };
    let o = c.offset(-COLONY / 2, -COLONY / 2);
    let across = COLONY / ROOM + 1;
    for room in (0..across * across).step_by(4) {
        let lamp = lamps[room as usize / 4 % lamps.len()];
        let p = o.offset((room % across) * ROOM + 5, (room / across) * ROOM + 5);
        if !s.world.map.inb(p) || s.world.map.fixture_at(p).is_some() {
            continue;
        }
        if let Some(e) = s.world.map.item_at(p) {
            s.world.despawn_thing(e);
        }
        if let Some(e) = s.world.spawn_fixture_of(lamp as _, p, false, None) {
            let _ = s.world.ecs.insert_one(e, Owner(Faction::Player));
        }
    }
}

/// Every pawn, topped up to `PAWNS` with colonists, the richest figure,
/// moved on to a grid round `at` on the surface, every other one half way
/// through a step in one of four directions. For drawing only: the sim is
/// not stepped again.
fn crowd(s: &mut Sim, at: IVec) {
    let start = s.world.defs.start.as_ref().map(|st| st.creature_r);
    while let Some(def) = start.filter(|_| s.world.pawns.len() < PAWNS) {
        s.world.spawn_pawn(def, Faction::Player, at, None);
    }
    let rows = (PAWNS as i32 + CROWD_COLUMNS - 1) / CROWD_COLUMNS;
    let steps = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    for (i, &e) in s.world.pawns.clone().iter().enumerate() {
        let Ok(mut p) = s.world.ecs.get::<&mut Pawn>(e) else { continue };
        let i = i as i32;
        let (col, row) = (i % CROWD_COLUMNS, i / CROWD_COLUMNS);
        p.pos = IVec::at(at.x + (col - CROWD_COLUMNS / 2) * CROWD_PITCH, at.y + (row - rows / 2) * CROWD_PITCH, 0);
        p.active = true;
        p.next = (i % 2 == 0).then(|| {
            let (dx, dy) = steps[(i / 2 % 4) as usize];
            p.pos.offset(dx, dy)
        });
        p.step_ticks = p.step_ticks.max(2);
        p.progress = p.step_ticks / 2;
    }
}

/// The crowd at one zoom.
struct Crowd {
    zoom: f32,
    lod: crate::figures::Lod,
    /// Mean CPU of the pawns pass, µs.
    pawns: f64,
    /// Figures and parts drawn, and the batch's draw calls, on the last frame.
    shown: usize,
    parts: usize,
    calls: usize,
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
    /// The hour to set the clock to; None keeps it.
    hour: Option<f64>,
    /// Light the colony's rooms first (`light_the_colony`).
    lit: bool,
    /// Dig the stacked scene first (`stacked`), and show this level.
    level: Option<i32>,
    /// Lights moving round the view's centre, frame by frame.
    moving: usize,
}

const VIEWS: [View; 10] = [
    View {
        name: "whole map",
        zoom: None,
        storm: false,
        zooming: false,
        scale: 1.0,
        hour: None,
        lit: false,
        level: None,
        moving: 0,
    },
    View {
        name: "mid",
        zoom: Some(12.0),
        storm: false,
        zooming: false,
        scale: 1.0,
        hour: None,
        lit: false,
        level: None,
        moving: 0,
    },
    View {
        name: "close",
        zoom: Some(28.0),
        storm: false,
        zooming: false,
        scale: 1.0,
        hour: None,
        lit: false,
        level: None,
        moving: 0,
    },
    View {
        name: "storm",
        zoom: None,
        storm: true,
        zooming: false,
        scale: 1.0,
        hour: None,
        lit: false,
        level: None,
        moving: 0,
    },
    View {
        name: "zooming",
        zoom: Some(12.0),
        storm: false,
        zooming: true,
        scale: 1.0,
        hour: None,
        lit: false,
        level: None,
        moving: 0,
    },
    // The storm at half the pixels: what render scale saves the GPU.
    View {
        name: "storm 50%",
        zoom: None,
        storm: true,
        zooming: false,
        scale: 0.5,
        hour: None,
        lit: false,
        level: None,
        moving: 0,
    },
    // A level dug out below the colony (DESIGN.md §6d): the surface with
    // it showing through pits, then the level itself, nearly all rock.
    View {
        name: "stacked",
        zoom: None,
        storm: false,
        zooming: false,
        scale: 1.0,
        hour: None,
        lit: false,
        level: Some(0),
        moving: 0,
    },
    View {
        name: "below",
        zoom: None,
        storm: false,
        zooming: false,
        scale: 1.0,
        hour: None,
        lit: false,
        level: Some(-1),
        moving: 0,
    },
    // Dusk with 64 lights moving round the colony: the preset's cap of
    // them cast shadows, the rest glow (5a69f9c9).
    View {
        name: "moving",
        zoom: None,
        storm: false,
        zooming: false,
        scale: 1.0,
        hour: Some(17.25),
        lit: true,
        level: None,
        moving: 64,
    },
    // Long shadows and lit fires: the most the lighting does. The sun is
    // where the sim has it: at 17:15 on the bench's day, about 9° up.
    View {
        name: "dusk",
        zoom: None,
        storm: false,
        zooming: false,
        scale: 1.0,
        hour: Some(17.25),
        lit: true,
        level: None,
        moving: 0,
    },
];

/// Ticks between evaluations of outdoor terms, which the sky is made of.
const TERMS_EVERY: usize = 20;

/// `--side-by-side DIR` (3c65738f): the same close views under `medium` and
/// `flat`, as pictures to compare: the colony at noon and at dusk, and a
/// hut lit by its own fire at night, `<view>_<preset>.png` each.
async fn side_by_side(app: &mut App, time: &mut f64, dir: &Path, centre: IVec) {
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("render bench: could not create {}: {e}", dir.display());
        std::process::exit(2);
    }
    light_the_colony(&mut app.sim);
    pin(app, CLEAR);
    // The first lamp stands five cells into the colony's corner room.
    let hut = centre.offset(-COLONY / 2 + 5, -COLONY / 2 + 5);
    for (name, hour, at, zoom) in
        [("noon", 12.0, centre, 28.0), ("dusk", 17.25, centre, 28.0), ("hut", 22.0, hut, 48.0)]
    {
        app.sim.world.tick = tick_at_hour(app.sim.world.tick, hour);
        for _ in 0..TERMS_EVERY {
            app.sim.step();
        }
        (app.cam.x, app.cam.y, app.cam.zoom) = (at.x as f32 + 0.5, at.y as f32 + 0.5, zoom);
        for setting in [crate::quality::Setting::Shadows, crate::quality::Setting::Flat] {
            app.light.set(setting);
            app.light.adapt_now();
            for _ in 0..30 {
                draw_one(app, time, None).await;
            }
            let shot = dir.join(format!("{name}_{}.png", setting.name()));
            draw_one(app, time, Some(&shot)).await;
            println!("side by side: {}", shot.display());
        }
    }
}

/// Frames each view settles for under `flat` before the side-by-side.
const FLAT_WARM: usize = 10;

/// Frames per view that time each lighting pass on the GPU.
const GPU_FRAMES: usize = 30;
/// Frames that each rebuild every cached lighting result.
const REBUILDS: usize = 10;

/// The tick of `hour` on the current day, from `tick`.
fn tick_at_hour(tick: u64, hour: f64) -> u64 {
    let day = tick / TICKS_PER_DAY;
    // Tick 0 is 06:00.
    let into = (hour / 24.0 * TICKS_PER_DAY as f64) as u64 + TICKS_PER_DAY - TICKS_PER_DAY / 4;
    day * TICKS_PER_DAY + into % TICKS_PER_DAY
}

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
    /// Of them, the chunk meshes'.
    mesh_calls: usize,
    mesh_indices: usize,
    /// The worst frame's painting and joining of chunk meshes (µs): what a
    /// zoom or a change costs in one frame.
    mesh_worst: f64,
    particles: usize,
    /// Things drawn live, outside the chunk meshes, on the last frame.
    live: usize,
    /// Chunk meshes rebuilt over the measured frames.
    rebuilt: usize,
    /// Held to the budget: the whole map, and a zoom gesture through it.
    gated: bool,
    /// Per lighting pass, over the measured frames.
    passes: Vec<PassMean>,
    /// Per frame, the wall time from one frame's start to the next's, in
    /// ms: every pass, submit, the GPU and the present, and any wait.
    walls: Vec<f64>,
    /// The same view's frames again under the other lighting setting
    /// (`flat` against `shadows`, 08a5d182), so both are measured on one
    /// runner.
    other: Vec<(RenderTimes, f64, Option<f64>)>,
}

/// One lighting pass over a view's frames.
struct PassMean {
    name: &'static str,
    /// Mean CPU, µs.
    cpu: f64,
    /// Mean GPU from the timed frames, µs; None where GL has no timer.
    gpu: Option<f64>,
    /// Share of frames it did its work in.
    ran: f64,
    /// Most draw calls it made in a frame.
    draws: u32,
}

/// Mean CPU per lighting pass, and how often it ran, over `frames`; mean
/// GPU per pass over `waited`.
fn pass_means(frames: &[Vec<PassTime>], waited: &[Vec<PassTime>]) -> Vec<PassMean> {
    let names: Vec<&'static str> = frames.first().map_or(Vec::new(), |f| f.iter().map(|p| p.name).collect());
    names
        .into_iter()
        .map(|name| {
            let of = |fs: &[Vec<PassTime>]| -> Vec<PassTime> {
                fs.iter().filter_map(|f| f.iter().find(|p| p.name == name).copied()).collect()
            };
            let (cpu, gpu) = (of(frames), of(waited));
            let n = cpu.len().max(1) as f64;
            let gpus: Vec<f64> = gpu.iter().filter_map(|p| p.gpu_us).collect();
            PassMean {
                name,
                cpu: cpu.iter().map(|p| p.cpu_us).sum::<f64>() / n,
                gpu: (!gpus.is_empty()).then(|| gpus.iter().sum::<f64>() / gpus.len() as f64),
                ran: cpu.iter().filter(|p| p.ran).count() as f64 / n,
                draws: cpu.iter().map(|p| p.draws).max().unwrap_or(0),
            }
        })
        .collect()
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

    /// The whole frame: (p50, p99, max) of the wall time, and the hitches,
    /// frames over twice the median.
    fn whole(&self) -> (f64, f64, f64, usize) {
        let mut v = self.walls.clone();
        v.sort_by(f64::total_cmp);
        let p50 = pct(&v, 0.5);
        (p50, pct(&v, 0.99), v.last().copied().unwrap_or(0.0), v.iter().filter(|&&w| w > 2.0 * p50).count())
    }

    /// The part of the mean frame no pass accounts for: the present, and
    /// waiting on the GPU where GL gave no timing for it.
    fn rest_ms(&self) -> f64 {
        let busy = self.mean(|f| f.0.world() + f.0.ui + f.1) + self.gpu_ms().unwrap_or(0.0);
        (self.walls.iter().sum::<f64>() / self.walls.len().max(1) as f64 - busy).max(0.0)
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
/// `--sync`: finish the GPU's work before each present, so a frame's wall
/// time is its CPU and GPU time with no frames queued behind it. A frame
/// the GPU can't finish in time then reads as its cost, not as a hitch
/// every few frames when the queue drains.
static SYNC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

async fn draw_one(app: &mut App, time: &mut f64, shot: Option<&Path>) {
    *time += 1.0 / 60.0;
    frame(app, &RawInput { time: *time, ..Default::default() });
    render(app);
    if SYNC.load(std::sync::atomic::Ordering::Relaxed) {
        // SAFETY: waits for GL to finish what's been submitted; no state
        // changes.
        unsafe { miniquad::gl::glFinish() };
    }
    if let Some(p) = shot {
        get_screen_data().export_png(&p.to_string_lossy());
    }
    next_frame().await;
}

pub async fn run(mut app: App, args: &[String]) -> ! {
    let opt = |name: &str| args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone());
    let frames: usize = opt("--frames").and_then(|v| v.parse().ok()).unwrap_or(300);
    let check = args.iter().any(|a| a == "--check");
    SYNC.store(args.iter().any(|a| a == "--sync"), std::sync::atomic::Ordering::Relaxed);
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
    if let Some(dir) = opt("--side-by-side").map(PathBuf::from) {
        side_by_side(&mut app, &mut time, &dir, centre).await;
        std::process::exit(0);
    }

    let mut results = Vec::new();
    let mut dug = false;
    for v in &VIEWS {
        pin(&mut app, if v.storm { STORM } else { CLEAR });
        if v.level.is_some() && !dug {
            dug = true;
            // Beside the stamped colony, where a player would dig first.
            let at = centre.offset(COLONY / 2 + 4, -STACKED / 2);
            if stacked(&mut app.sim, at, STACKED).is_none() {
                eprintln!("render bench: the mods have no way down or no air to dig the stacked scene with");
            }
            // A river breaks into the room's west wall and floods it
            // (202b16c4), so the level below is measured with water drawn.
            let defs = app.sim.world.defs.clone();
            if let Some(river) = defs.terrain.iter().position(|t| t.pours > 0) {
                let wall = IVec::at(at.x - 1, at.y + STACKED / 2, -1);
                app.sim.world.map.set_terrain(wall, river as rim_sim::defs::DefId, 0);
                for _ in 0..2_000 {
                    app.sim.step();
                }
            }
        }
        app.cam.z = v.level.unwrap_or(0);
        if v.lit {
            light_the_colony(&mut app.sim);
        }
        // Pinned channels reach the world's outdoor values on a tick; the
        // hour's terms (the sky) are evaluated every 20.
        let settle = match v.hour {
            Some(h) => {
                app.sim.world.tick = tick_at_hour(app.sim.world.tick, h);
                TERMS_EVERY
            }
            None => 1,
        };
        for _ in 0..settle {
            app.sim.step();
        }
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
        // Moving lights circle the colony, a little on each frame.
        let around = vec2(centre.x as f32 + 0.5, centre.y as f32 + 0.5);
        let circling = |k: usize| {
            (0..v.moving).map(move |i| {
                let (r, a) = (6.0 + 12.0 * i as f32 / v.moving as f32, i as f32 * 0.7 + k as f32 * 0.02);
                (around + r * vec2(a.cos(), a.sin()), 4.0, 60.0)
            })
        };
        for k in 0..WARM {
            if v.zooming {
                app.cam.zoom = gesture_zoom(k);
            }
            app.light.set_moving(circling(k));
            draw_one(&mut app, &mut time, None).await;
        }
        let mut r =
            Run { name: v.name, zoom: app.cam.zoom, gated: v.zoom.is_none() || v.zooming, ..Default::default() };
        let mut passes = Vec::with_capacity(frames);
        for k in WARM..WARM + frames {
            if v.zooming {
                app.cam.zoom = gesture_zoom(k);
            }
            app.light.set_moving(circling(k));
            let t0 = std::time::Instant::now();
            draw_one(&mut app, &mut time, None).await;
            r.walls.push(t0.elapsed().as_secs_f64() * 1e3);
            r.rebuilt += app.meshes.rebuilt;
            r.mesh_worst = r.mesh_worst.max(app.meshes.paint_us + app.meshes.join_us);
            let zones = telemetry::frame().zones;
            // Submit: macroquad's end of frame, and the meshes' mid-frame.
            let submit = zone(&zones, "Event::draw end_frame").unwrap_or(0.0) + app.render_us.gl;
            r.frames.push((app.render_us, submit, zone(&zones, "glFinish/glFLush")));
            passes.push(app.light.passes.clone());
        }
        // A gesture's numbers are for the zoom it ended on.
        r.zoom = app.cam.zoom;
        // The capture is taken on the frame after the one that asks.
        telemetry::capture_frame();
        draw_one(&mut app, &mut time, None).await;
        let shot = shots.as_ref().map(|d| d.join(format!("{}.png", v.name.replace(' ', "_").replace('%', ""))));
        draw_one(&mut app, &mut time, shot.as_deref()).await;
        // Macroquad's capture sees its own batches; the chunk meshes and the
        // pawns' figures are drawn past it and count themselves.
        let calls = telemetry::drawcalls();
        r.calls = calls.len() + app.meshes.calls + app.figures.calls;
        r.indices = calls.iter().map(|c| c.indices_count).sum::<usize>() + app.meshes.indices + app.figures.indices();
        (r.mesh_calls, r.mesh_indices) = (app.meshes.calls, app.meshes.indices);
        r.particles = app.sky.particles();
        r.live = app.meshes.live_count();
        // The lighting passes' GPU time, on frames of their own after
        // everything above, which they would disturb.
        app.light.time_gpu(true);
        let mut waited = Vec::with_capacity(GPU_FRAMES);
        for _ in 0..GPU_FRAMES {
            draw_one(&mut app, &mut time, None).await;
            waited.push(app.light.passes.clone());
        }
        app.light.time_gpu(false);
        r.passes = pass_means(&passes, &waited);
        // The other setting: the same frames, moving lights and all, then
        // back to the one the bench runs.
        {
            let was = *app.light.setting();
            app.light.set(if was.flat() { crate::quality::Setting::Shadows } else { crate::quality::Setting::Flat });
            // A fifth of the frames: CI's client job has no room for twice
            // the bench, and a mean of twenty holds.
            for k in 0..FLAT_WARM {
                app.light.set_moving(circling(k));
                draw_one(&mut app, &mut time, None).await;
            }
            for k in FLAT_WARM..FLAT_WARM + (frames / 5).max(10) {
                if v.zooming {
                    app.cam.zoom = gesture_zoom(k);
                }
                app.light.set_moving(circling(k));
                draw_one(&mut app, &mut time, None).await;
                let zones = telemetry::frame().zones;
                let submit = zone(&zones, "Event::draw end_frame").unwrap_or(0.0) + app.render_us.gl;
                r.other.push((app.render_us, submit, zone(&zones, "glFinish/glFLush")));
            }
            let name = format!("{}_{}.png", v.name.replace(' ', "_").replace('%', ""), app.light.setting().name());
            let shot = shots.as_ref().map(|d| d.join(name));
            draw_one(&mut app, &mut time, shot.as_deref()).await;
            app.light.set(was);
        }
        results.push(r);
    }

    // What a change costs: every cached lighting result rebuilt, on the
    // view the loop ended on, a frame at a time. Only the passes that cache:
    // one that runs every frame costs the same whether anything changed.
    let last = results.last().map_or("", |r: &Run| r.name);
    let cached: Vec<&'static str> =
        results.last().map_or(Vec::new(), |r| r.passes.iter().filter(|p| p.ran < 1.0).map(|p| p.name).collect());
    app.light.time_gpu(true);
    let mut rebuilds: Vec<Vec<PassTime>> = Vec::with_capacity(REBUILDS);
    for _ in 0..REBUILDS {
        app.light.invalidate();
        draw_one(&mut app, &mut time, None).await;
        rebuilds.push(app.light.passes.iter().filter(|p| cached.contains(&p.name)).copied().collect());
    }
    app.light.time_gpu(false);
    let rebuilt = pass_means(&rebuilds, &rebuilds);
    // And a change to the map: the colony's chunk painted again, and its
    // region joined, as building a wall there does.
    let at = app.sim.world.map.chunk_of(centre);
    let (mut paint, mut join) = (0.0, 0.0);
    for _ in 0..REBUILDS {
        app.meshes.repaint(at);
        draw_one(&mut app, &mut time, None).await;
        (paint, join) = (paint + app.meshes.paint_us, join + app.meshes.join_us);
    }
    let (paint, join) = (paint / REBUILDS as f64 / 1e3, join / REBUILDS as f64 / 1e3);

    // Last, as it moves every pawn: the crowd.
    crowd(&mut app.sim, centre);
    pin(&mut app, CLEAR);
    app.cam.z = 0;
    app.cam.x = centre.x as f32 + 0.5;
    app.cam.y = centre.y as f32 + 0.5;
    app.render_scale = Some(1.0);
    app.motion.face(&app.sim.world, &app.worksites, 0.0);
    let mut crowds = Vec::new();
    for zoom in CROWD_ZOOMS {
        app.cam.zoom = zoom;
        for _ in 0..10 {
            draw_one(&mut app, &mut time, None).await;
        }
        let mut sum = 0.0;
        for _ in 0..frames {
            draw_one(&mut app, &mut time, None).await;
            sum += app.render_us.pawns;
        }
        let shot = shots.as_ref().map(|d| d.join(format!("crowd_{zoom}.png")));
        draw_one(&mut app, &mut time, shot.as_deref()).await;
        crowds.push(Crowd {
            zoom,
            lod: crate::figures::lod(zoom),
            pawns: sum / frames.max(1) as f64,
            shown: app.figures_shown.len(),
            parts: app.figures.len(),
            calls: app.figures.calls,
        });
    }
    let renderer = crate::light::gl_renderer();

    println!(
        "render bench: {SIZE}×{SIZE}, {} colonists, {} pawns, {}×{} points at {dpi}x, {frames} frames per view",
        app.sim.world.colonists().count(),
        app.sim.world.pawns.len(),
        screen_width(),
        screen_height()
    );
    println!(
        "{:<10} {:>5} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>6} {:>5} {:>8} {:>7} {:>6}",
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
        "mesh",
        "indices",
        "rebuilt",
        "live"
    );
    for r in &results {
        let sorted = r.world_ms();
        println!(
            "{:<10} {:>5.0} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7} {:>6} {:>5} {:>8} {:>7} {:>6}",
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
            r.mesh_calls,
            r.indices,
            r.rebuilt,
            r.live
        );
    }
    println!("ms per frame, CPU unless named; world = every pass but the UI; mesh = the chunk meshes' calls; budgets in budgets.toml (render.gpu, or render.software under software GL)");
    println!();
    println!("{:<10} {:>7} {:>7} {:>7} {:>7} {:>7}", "view", "frame", "p99", "max", "hitches", "rest");
    for r in &results {
        let (p50, p99, max, hitches) = r.whole();
        println!("{:<10} {:>7.3} {:>7.3} {:>7.3} {:>7} {:>7.3}", r.name, p50, p99, max, hitches, r.rest_ms());
    }
    println!(
        "whole frame, wall ms from one frame's start to the next: median, p99, worst; hitches = frames over 2x the median; \
         rest = the mean frame less every pass, submit and gpu (the present, and waits)"
    );
    println!();
    println!("{:<10} {:<10} {:>7} {:>7} {:>5} {:>6}", "view", "light pass", "cpu", "gpu", "ran", "calls");
    for r in &results {
        for p in &r.passes {
            let gpu = p.gpu.map_or("-".into(), |g| format!("{:.3}", g / 1e3));
            println!(
                "{:<10} {:<10} {:>7.3} {:>7} {:>4.0}% {:>6}",
                r.name,
                p.name,
                p.cpu / 1e3,
                gpu,
                p.ran * 100.0,
                p.draws
            );
        }
    }
    println!("ms per frame; gpu from a timer query over {GPU_FRAMES} more frames, - where GL has none; ran = frames it did work");
    let costs: Vec<String> = rebuilt
        .iter()
        .map(|p| {
            let gpu = p.gpu.map_or("-".into(), |g| format!("{:.3}", g / 1e3));
            format!("{} {:.3} cpu, {gpu} gpu", p.name, p.cpu / 1e3)
        })
        .collect();
    println!("light rebuild on {last}, ms: {}", costs.join("; "));
    println!("mesh change on {last}, ms: paint {paint:.3}, join {join:.3}");
    println!();
    println!(
        "{:<10} {:>5} {:<10} {:>7} {:>7} {:>7} {:>6}",
        "crowd", "zoom", "detail", "pawns", "figures", "parts", "calls"
    );
    for c in &crowds {
        println!(
            "{:<10} {:>5.0} {:<10} {:>7.3} {:>7} {:>7} {:>6}",
            "",
            c.zoom,
            format!("{:?}", c.lod).to_lowercase(),
            c.pawns / 1e3,
            c.shown,
            c.parts,
            c.calls
        );
    }
    println!("ms per frame of the pawns pass's CPU; budget {FIGURES_MS} ms for {PAWNS} in full, in one call");
    let soft = if crate::light::software_gl(&renderer) { " (software: gpu times are the CPU rasterising)" } else { "" };
    println!("gl: {renderer}{soft}");
    println!("machine: {}", machine());
    if SYNC.load(std::sync::atomic::Ordering::Relaxed) {
        println!("sync: glFinish before each present, so a frame is its CPU and GPU time with nothing queued");
    }
    println!("lighting: {}", app.light.setting().name());
    if results.iter().any(|r| !r.other.is_empty()) {
        let (this, other) = (app.light.setting().name(), if app.light.setting().flat() { "shadows" } else { "flat" });
        println!("\nlighting side-by-side, ms per frame: {this} against {other}, the same frames");
        println!(
            "{:<10} {:>8} {:>8} {:>8} {:>8}   {:>8} {:>8} {:>8} {:>8}",
            "view", "world", "light", "submit", "gpu", "world", "light", "submit", "gpu"
        );
        for r in &results {
            let flat = Run { other: Vec::new(), frames: r.other.clone(), ..Default::default() };
            let gpu = |g: Option<f64>| g.map_or("-".into(), |g| format!("{g:.3}"));
            println!(
                "{:<10} {:>8.3} {:>8.3} {:>8.3} {:>8}   {:>8.3} {:>8.3} {:>8.3} {:>8}",
                r.name,
                r.mean(|f| f.0.world()),
                r.mean(|f| f.0.light),
                r.mean(|f| f.1),
                gpu(r.gpu_ms()),
                flat.mean(|f| f.0.world()),
                flat.mean(|f| f.0.light),
                flat.mean(|f| f.1),
                gpu(flat.gpu_ms()),
            );
        }
    }

    if let Some(path) = opt("--json") {
        let views: Vec<String> = results
            .iter()
            .map(|r| {
                let sorted = r.world_ms();
                let passes: Vec<String> = r
                    .passes
                    .iter()
                    .map(|p| {
                        format!(
                            "{{\"name\": \"{}\", \"cpu_ms\": {:.4}, \"gpu_ms\": {}, \"ran\": {:.3}, \"draw_calls\": {}}}",
                            p.name,
                            p.cpu / 1e3,
                            p.gpu.map_or("null".into(), |g| format!("{:.4}", g / 1e3)),
                            p.ran,
                            p.draws
                        )
                    })
                    .collect();
                let (p50, p99, max, hitches) = r.whole();
                let whole = format!(
                    "\"frame_p50_ms\": {p50:.4}, \"frame_p99_ms\": {p99:.4}, \"frame_max_ms\": {max:.4}, \"hitches\": {hitches}, \"rest_ms\": {:.4}, ",
                    r.rest_ms()
                );
                format!(
                    "    {{\"view\": \"{}\", \"zoom\": {}, {whole}\"world_ms\": {:.4}, \"world_p50_ms\": {:.4}, \"world_p99_ms\": {:.4}, \"ground_ms\": {:.4}, \"things_ms\": {:.4}, \"pawns_ms\": {:.4}, \"weather_ms\": {:.4}, \"light_ms\": {:.4}, \"ui_ms\": {:.4}, \"submit_ms\": {:.4}, \"gpu_ms\": {}, \"draw_calls\": {}, \"indices\": {}, \"mesh_draw_calls\": {}, \"mesh_indices\": {}, \"mesh_worst_ms\": {:.4}, \"particles\": {}, \"rebuilt\": {}, \"live\": {}, \"light_passes\": [{}]{}}}",
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
                    r.mesh_calls,
                    r.mesh_indices,
                    r.mesh_worst / 1e3,
                    r.particles,
                    r.rebuilt,
                    r.live,
                    passes.join(", "),
                    if r.other.is_empty() {
                        String::new()
                    } else {
                        let flat = Run { frames: r.other.clone(), ..Default::default() };
                        format!(
                            ", \"other\": {{\"world_ms\": {:.4}, \"light_ms\": {:.4}, \"submit_ms\": {:.4}, \"gpu_ms\": {}}}",
                            flat.mean(|f| f.0.world()),
                            flat.mean(|f| f.0.light),
                            flat.mean(|f| f.1),
                            flat.gpu_ms().map_or("null".into(), |g| format!("{g:.4}"))
                        )
                    }
                )
            })
            .collect();
        let rebuild: Vec<String> = rebuilt
            .iter()
            .map(|p| {
                let gpu = p.gpu.map_or("null".into(), |g| format!("{:.4}", g / 1e3));
                format!("\"{}\": {{\"cpu_ms\": {:.4}, \"gpu_ms\": {gpu}}}", p.name, p.cpu / 1e3)
            })
            .collect();
        let crowd: Vec<String> = crowds
            .iter()
            .map(|c| {
                format!(
                    "    {{\"zoom\": {}, \"detail\": \"{}\", \"pawns_ms\": {:.4}, \"figures\": {}, \"parts\": {}, \"draw_calls\": {}}}",
                    c.zoom,
                    format!("{:?}", c.lod).to_lowercase(),
                    c.pawns / 1e3,
                    c.shown,
                    c.parts,
                    c.calls
                )
            })
            .collect();
        let json = format!(
            "{{\n  \"machine\": \"{}\",\n  \"screen\": [{}, {}],\n  \"dpi\": {dpi},\n  \"frames\": {frames},\n  \"figures_budget_ms\": {FIGURES_MS},\n  \"crowd\": [\n{}\n  ],\n  \"gl_renderer\": \"{}\",\n  \"lighting\": \"{}\",\n  \"light_rebuild_view\": \"{last}\",\n  \"light_rebuild\": {{{}}},\n  \"mesh_change\": {{\"paint_ms\": {paint:.4}, \"join_ms\": {join:.4}}},\n  \"views\": [\n{}\n  ]\n}}\n",
            machine().replace('"', "'"),
            screen_width(),
            screen_height(),
            crowd.join(",\n"),
            renderer.replace('"', "'"),
            app.light.setting().name(),
            rebuild.join(", "),
            views.join(",\n")
        );
        if let Err(e) = std::fs::write(&path, json) {
            eprintln!("render bench: could not write {path}: {e}");
            std::process::exit(2);
        }
    }

    if check {
        // budgets.toml's render.gpu or render.software, each measure the
        // worst view's; world CPU over the gated views only. Only the
        // default lighting is held: the other setting's reruns are reported,
        // and counting them too would double the draw calls.
        let budgets = rim_sim::budgets::Budgets::repo().unwrap_or_else(|e| {
            eprintln!("render bench: {e}");
            std::process::exit(2);
        });
        let slack = budgets.slack(&machine(), std::env::var_os("CI").is_some());
        let section = if crate::light::software_gl(&renderer) { "software" } else { "gpu" };
        let worst = |f: &dyn Fn(&Run) -> f64, gated: bool| {
            results.iter().filter(|r| !gated || r.gated).map(f).fold(0.0, f64::max)
        };
        let measured = [
            ("world_ms", worst(&|r| r.mean(|f| f.0.world()), true)),
            ("frame_p99_ms", worst(&|r| r.whole().1, false)),
            ("frame_max_ms", worst(&|r| r.whole().2, false)),
            ("hitches", worst(&|r| r.whole().3 as f64, false)),
            ("draw_calls", worst(&|r| r.calls as f64, false)),
            // What a new wall costs the frame it goes up in.
            ("mesh_change_ms", paint + join),
        ];
        let over = rim_sim::budgets::Budgets::over(&budgets.render, section, &measured, slack).unwrap_or_else(|e| {
            eprintln!("render bench: {e}");
            std::process::exit(2);
        });
        if !over.is_empty() {
            for o in &over {
                eprintln!("render bench: render.{o}");
            }
            std::process::exit(1);
        }
        println!("render bench: render.{section} within budgets.toml (slack {slack})");
        let figures = FIGURES_MS * slack;
        for c in &crowds {
            if c.calls > 1 {
                eprintln!("render bench: the crowd at {} takes {} draw calls, not one", c.zoom, c.calls);
                std::process::exit(1);
            }
        }
        let Some(full) = crowds.iter().find(|c| c.lod == crate::figures::Lod::Full) else {
            unreachable!("CROWD_ZOOMS has a zoom in full")
        };
        if full.shown < PAWNS {
            eprintln!("render bench: only {} of the crowd's {PAWNS} figures are on screen in full", full.shown);
            std::process::exit(1);
        }
        let ms = full.pawns / 1e3;
        if ms > figures {
            eprintln!(
                "render bench: {PAWNS} figures in full take {ms:.3} ms of CPU, over the budget of {figures:.2} ms"
            );
            std::process::exit(1);
        }
        println!("render bench: figures within budget ({PAWNS} in full: {ms:.3} <= {figures:.2} ms, one call)");
    }
    std::process::exit(0)
}

fn machine() -> String {
    rim_sim::budgets::machine()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crowd_is_200_on_one_level_and_fits_the_screen_in_full() {
        let mut s = Sim::new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods"), 1).expect("the mods load");
        let at = IVec::new(100, 100);
        crowd(&mut s, at);
        assert_eq!(s.world.pawns.len(), PAWNS);
        let cells: Vec<IVec> = s.world.pawns.iter().map(|&e| s.world.ecs.get::<&Pawn>(e).unwrap().pos).collect();
        assert!(cells.iter().all(|c| c.z == 0));
        let span = |f: fn(&IVec) -> i32| cells.iter().map(f).max().unwrap() - cells.iter().map(f).min().unwrap() + 1;
        // The reference screen at 40 points a cell is 48 by 27 cells.
        assert!(
            span(|c| c.x) <= 1920 / 40 - 2 && span(|c| c.y) <= 1080 / 40 - 2,
            "{} by {}",
            span(|c| c.x),
            span(|c| c.y)
        );
        let walking = s.world.pawns.iter().filter(|&&e| s.world.ecs.get::<&Pawn>(e).unwrap().next.is_some()).count();
        assert_eq!(walking, PAWNS / 2, "every other one mid-step");
    }

    #[test]
    fn the_dusk_view_sets_the_clock_to_its_hour_on_the_same_day() {
        for tick in [0, 7_777, 3 * TICKS_PER_DAY + 19_999] {
            let t = tick_at_hour(tick, 18.67);
            assert_eq!(t / TICKS_PER_DAY, tick / TICKS_PER_DAY, "same day");
            assert!((rim_sim::world::hour_at(t) - 18.67).abs() < 0.01, "{}", rim_sim::world::hour_at(t));
        }
    }
}
