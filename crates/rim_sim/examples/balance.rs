//! Balance harness: a simple bot plays the opening on many seeds and reports
//! how the first days go. The bot acts only through Commands, like a player.
//!
//!   cargo run --release -p rim_sim --example balance -- --seeds 20 --days 5
//!
//! The bot: designate trees and berry bushes near the start, then build a
//! 5x5 wooden hut (walls, a door, a bed inside). Colonists defend themselves.
//!
//! Flags: `--nohut`, `--fire` (a campfire in the hut), `--cold-snap DAY`, `--core` (core alone,
//! no weather plugin), `--start-day N` (start on day N of the year, by adding
//! a patch mod to a copy of the mods folder), `--show SEED` (print that run's
//! messages). Runs over more than one season also report each season. Seeds
//! run in parallel.

use rim_sim::world::{Blueprint, MsgKind, Pawn, NEED_MAX};
use rim_sim::{Command, IVec, Sim, TICKS_PER_DAY};
use std::path::Path;

fn arg(name: &str, default: u64) -> u64 {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(default)
}

#[derive(Default)]
struct Report {
    seed: u64,
    survived: bool,
    colonists_end: usize,
    deaths: Vec<(f64, String)>,
    min_hp: f64,
    min_food: f64,
    min_warmth: f64,
    /// Lowest warmth after the first night, once there's been time to build.
    min_warmth_later: f64,
    /// Colonist-hours spent at zero warmth (taking hypothermia damage).
    frozen_ticks: u64,
    /// The founder's hours at zero warmth after night one: what a one-bed hut is for.
    founder_frozen_later: u64,
    starving_ticks: u64,
    hut_done_day: Option<f64>,
    threats: Vec<(f64, String)>,
    goods: usize,
    wealth: f64,
    /// Colonist-hours at zero warmth, per season index.
    frozen_by_season: Vec<f64>,
    deaths_by_season: Vec<u32>,
    /// Colonists alive at the end of each season.
    alive_by_season: Vec<usize>,
    /// The founder's hours at zero warmth, per season, and whether they lived.
    founder_frozen_by_season: Vec<f64>,
    founder_alive: bool,
}

fn open_square(s: &Sim, c: IVec, size: i32) -> Option<IVec> {
    let m = &s.world.map;
    let free = |p: IVec| m.passable(p) && m.fixture_at(p).is_none() && m.cost(p) <= 100;
    for r in 1..30i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                let o = c.offset(dx, dy);
                if (0..size).all(|y| (0..size).all(|x| free(o.offset(x, y)))) {
                    return Some(o);
                }
            }
        }
    }
    None
}

fn play(mods: &Path, seed: u64, days: u64) -> Report {
    let core = std::env::args().any(|a| a == "--core");
    let mut s = Sim::with_mods(mods, seed, &|m| !core || m == "core").expect("mods load");
    let seasons = s.world.defs.calendar.seasons.len();
    let defs = s.world.defs.clone();
    let c = s.world.colony_center().unwrap();
    let des = |id: &str| defs.lookup("designation", id).unwrap();
    let thing = |id: &str| defs.thing_id(id).unwrap();
    let food = defs.lookup("need", "food").unwrap();
    let warmth = defs.lookup("need", "warmth");

    s.push(Command::Designate { designation: des("chop"), a: c.offset(-14, -14), b: c.offset(14, 14) });
    s.push(Command::Designate { designation: des("harvest"), a: c.offset(-20, -20), b: c.offset(20, 20) });
    let hut = if std::env::args().any(|a| a == "--nohut") { None } else { open_square(&s, c, 5) };
    if let Some(o) = hut {
        let (w, d, b) = (thing("wall"), thing("door"), thing("bed"));
        let wood = thing("wood");
        let door = o.offset(2, 4);
        for y in 0..5 {
            for x in 0..5 {
                let p = o.offset(x, y);
                let edge = x == 0 || y == 0 || x == 4 || y == 4;
                if edge && p != door {
                    s.push(Command::Build { stuff: Some(wood), thing: w, a: p, b: p });
                }
            }
        }
        s.push(Command::Build { stuff: Some(wood), thing: d, a: door, b: door });
        s.push(Command::Build { stuff: Some(wood), thing: b, a: o.offset(2, 2), b: o.offset(2, 2) });
        if std::env::args().any(|a| a == "--fire") {
            let f = thing("campfire");
            s.push(Command::Build { stuff: None, thing: f, a: o.offset(1, 1), b: o.offset(1, 1) });
        }
    }

    let mut r = Report {
        seed,
        min_hp: 1.0,
        min_food: 1.0,
        min_warmth: 1.0,
        min_warmth_later: 1.0,
        frozen_by_season: vec![0.0; seasons],
        deaths_by_season: vec![0; seasons],
        alive_by_season: vec![0; seasons],
        founder_frozen_by_season: vec![0.0; seasons],
        ..Default::default()
    };
    let mut seen = 0;
    // --cold-snap DAY [--snap-drop C]: a cold snap on that day (C°C, default 8, for
    // two days), pushed directly so every run gets one.
    let snap = std::env::args().any(|a| a == "--cold-snap").then(|| arg("--cold-snap", 5) * TICKS_PER_DAY);
    for _ in 0..days * TICKS_PER_DAY {
        if snap == Some(s.world.tick) {
            let t = defs.lookup("field", "temperature").unwrap() as usize;
            let now = s.world.tick;
            s.world.fields.push_ambient(t, "cold_snap", -(arg("--snap-drop", 8) as f64), now, Some(48.0), 3.0);
        }
        s.step();
        let w = &s.world;
        let day = w.tick as f64 / TICKS_PER_DAY as f64;
        if w.tick.is_multiple_of(60) {
            for e in w.colonists() {
                let p = w.ecs.get::<&Pawn>(e).unwrap();
                let max = defs.creature(p.def).max_hp as f64;
                r.min_hp = r.min_hp.min(p.hp as f64 / max);
                if let Some(v) = warmth.and_then(|n| p.need(n)) {
                    r.min_warmth = r.min_warmth.min(v as f64 / NEED_MAX as f64);
                    if day >= 1.4 {
                        r.min_warmth_later = r.min_warmth_later.min(v as f64 / NEED_MAX as f64);
                    }
                    if v == 0 {
                        r.frozen_by_season[w.season_index() as usize] += 60.0 * 24.0 / TICKS_PER_DAY as f64;
                        r.frozen_ticks += 60;
                        if p.founder && day >= 1.4 {
                            r.founder_frozen_later += 60;
                        }
                        if p.founder {
                            r.founder_frozen_by_season[w.season_index() as usize] += 60.0 * 24.0 / TICKS_PER_DAY as f64;
                        }
                    }
                }
                if let Some(f) = p.need(food) {
                    r.min_food = r.min_food.min(f as f64 / NEED_MAX as f64);
                    if f == 0 {
                        r.starving_ticks += 60;
                    }
                }
            }
            if hut.is_some() && r.hut_done_day.is_none() && w.ecs.query::<&Blueprint>().iter().next().is_none() {
                r.hut_done_day = Some(day);
            }
        }
        for m in &w.messages[seen..] {
            if arg("--show", 0) == seed {
                println!("  d{day:.2} [{:?}] {}", m.kind, m.text);
            }
            match m.kind {
                MsgKind::Threat => r.threats.push((day, m.text.clone())),
                MsgKind::Bad if m.text.ends_with("has died.") => {
                    r.deaths.push((day, m.text.clone()));
                    r.deaths_by_season[w.season_index() as usize] += 1;
                }
                MsgKind::Good => r.goods += 1,
                _ => {}
            }
        }
        seen = w.messages.len();
        let si = w.season_index() as usize;
        r.alive_by_season[si] = w.colonists().count();
        if w.colony_lost {
            break;
        }
    }
    let w = &s.world;
    r.colonists_end = w.colonists().count();
    r.founder_alive = w.colonists().any(|e| w.ecs.get::<&Pawn>(e).is_ok_and(|p| p.founder));
    r.survived = r.colonists_end > 0;
    r.wealth = w.wealth;
    r
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

fn main() {
    let seeds = arg("--seeds", 20);
    let days = arg("--days", 5);
    let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    // A later start is a mod like any other: a patch to core's calendar.
    let start = std::env::args().any(|a| a == "--start-day").then(|| arg("--start-day", 0));
    let mods = match start {
        Some(day) => {
            let dir = std::env::temp_dir().join(format!("rim-balance-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            copy_dir(&shipped, &dir);
            let m = dir.join("balance_start");
            std::fs::create_dir_all(m.join("defs")).unwrap();
            std::fs::write(
                m.join("mod.toml"),
                "id = \"balance_start\"\nname = \"Balance start\"\nversion = \"0.1.0\"\napi = \"0.1\"\ndepends = [\"core\"]\n",
            )
            .unwrap();
            std::fs::write(
                m.join("defs/start.toml"),
                format!("[[patch]]\ntarget = \"calendar/core\"\nset = {{ start_day = {day} }}\n"),
            )
            .unwrap();
            dir
        }
        None => shipped,
    };
    // Seeds in parallel: each run builds its own Sim on its own thread.
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()) as u64;
    let mut reports: Vec<Report> = std::thread::scope(|sc| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let mods = &mods;
                sc.spawn(move || {
                    (1..=seeds).filter(|s| s % threads == t).map(|seed| play(mods, seed, days)).collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    reports.sort_by_key(|r| r.seed);

    println!("seed  alive  end  min_hp  min_food  min_warm  starved_h  hut_day  threats  good  wealth  deaths");
    for r in &reports {
        let starved_h = r.starving_ticks as f64 / TICKS_PER_DAY as f64 * 24.0;
        let hut = r.hut_done_day.map_or("-".into(), |d| format!("{d:.2}"));
        let deaths: Vec<String> = r.deaths.iter().map(|(d, t)| format!("d{d:.1} {t}")).collect();
        println!(
            "{:>4}  {:>5}  {:>3}  {:>6.2}  {:>8.2}  {:>8.2}  {:>9.1}  {:>7}  {:>7}  {:>4}  {:>6.0}  {}",
            r.seed,
            if r.survived { "yes" } else { "NO" },
            r.colonists_end,
            r.min_hp,
            r.min_food,
            r.min_warmth,
            starved_h,
            hut,
            r.threats.len(),
            r.goods,
            r.wealth,
            deaths.join("; ")
        );
    }
    let n = reports.len() as f64;
    let lost = reports.iter().filter(|r| !r.survived).count();
    let any_death = reports.iter().filter(|r| !r.deaths.is_empty()).count();
    let near_miss = reports.iter().filter(|r| r.survived && r.min_hp < 0.35).count();
    let hungry = reports.iter().filter(|r| r.min_food < 0.1).count();
    let no_hut = reports.iter().filter(|r| r.hut_done_day.is_none_or(|d| d > 1.0)).count();
    let first_threat: Vec<f64> = reports.iter().filter_map(|r| r.threats.first().map(|t| t.0)).collect();
    println!();
    println!("colonies lost:            {lost}/{n}");
    println!("runs with a death:        {any_death}/{n}");
    println!("near-misses (hp < 35%):   {near_miss}/{n}");
    println!("food ever below 10%:      {hungry}/{n}");
    let cold = reports.iter().filter(|r| r.min_warmth < 0.1).count();
    println!("warmth ever below 10%:    {cold}/{n}");
    let later = reports.iter().map(|r| r.min_warmth_later).sum::<f64>() / n;
    let frozen = reports.iter().filter(|r| r.frozen_ticks > 0).count();
    let frozen_h = reports.iter().map(|r| r.frozen_ticks).sum::<u64>() as f64 / TICKS_PER_DAY as f64 * 24.0 / n;
    println!("warmth after night one:   mean low {later:.2}");
    println!("froze (warmth hit 0):     {frozen}/{n} runs, {frozen_h:.1} colonist-hours per run");
    let ff = reports.iter().filter(|r| r.founder_frozen_later > 0).count();
    let ff_h = reports.iter().map(|r| r.founder_frozen_later).sum::<u64>() as f64 / TICKS_PER_DAY as f64 * 24.0 / n;
    println!("founder froze after n1:   {ff}/{n} runs, {ff_h:.1} hours per run");
    println!("hut not done by day 1:    {no_hut}/{n}");
    if !first_threat.is_empty() {
        let mean = first_threat.iter().sum::<f64>() / first_threat.len() as f64;
        let min = first_threat.iter().cloned().fold(f64::MAX, f64::min);
        println!("first threat day:         mean {mean:.2}, earliest {min:.2} ({} runs had one)", first_threat.len());
    }
    // Per season, over runs that reached it.
    if days > 15 {
        let names = {
            let core = std::env::args().any(|a| a == "--core");
            let s = Sim::with_mods(&mods, 1, &|m| !core || m == "core").expect("mods load");
            s.world.defs.calendar.seasons.clone()
        };
        println!("\nseason    frozen colonist-h/run  founder frozen h/run  deaths  colonies alive at end");
        for (i, name) in names.iter().enumerate() {
            let frozen = reports.iter().map(|r| r.frozen_by_season[i]).sum::<f64>() / n;
            let founder = reports.iter().map(|r| r.founder_frozen_by_season[i]).sum::<f64>() / n;
            let deaths: u32 = reports.iter().map(|r| r.deaths_by_season[i]).sum();
            let alive = reports.iter().filter(|r| r.alive_by_season[i] > 0).count();
            println!("{name:<9} {frozen:>22.1}  {founder:>20.1}  {deaths:>6}  {alive:>6}/{n}");
        }
        let founders = reports.iter().filter(|r| r.founder_alive).count();
        println!("founder alive at the end: {founders}/{n}");
    }
    let threat_kinds: Vec<&str> = reports.iter().flat_map(|r| r.threats.iter().map(|t| t.1.as_str())).collect();
    for t in threat_kinds.iter().take(12) {
        println!("  {t}");
    }
}
