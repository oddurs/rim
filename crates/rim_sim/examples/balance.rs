//! Balance harness: a simple bot plays the opening on many seeds and reports
//! how the first days go. The bot acts only through Commands, like a player.
//!
//!   cargo run --release -p rim_sim --example balance -- --seeds 20 --days 5
//!
//! The bot: designate trees and berry bushes near the start, then build a
//! 5x5 wooden hut (walls, a door, a bed inside). Colonists defend themselves.

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
    starving_ticks: u64,
    hut_done_day: Option<f64>,
    threats: Vec<(f64, String)>,
    goods: usize,
    wealth: f64,
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
    let mut s = Sim::new(mods, seed).expect("mods load");
    let defs = s.world.defs.clone();
    let c = s.world.colony_center().unwrap();
    let des = |id: &str| defs.lookup("designation", id).unwrap();
    let thing = |id: &str| defs.thing_id(id).unwrap();
    let food = defs.lookup("need", "food").unwrap();

    s.push(Command::Designate { designation: des("chop"), a: c.offset(-14, -14), b: c.offset(14, 14) });
    s.push(Command::Designate { designation: des("harvest"), a: c.offset(-20, -20), b: c.offset(20, 20) });
    let hut = open_square(&s, c, 5);
    if let Some(o) = hut {
        let (w, d, b) = (thing("wall_wood"), thing("door_wood"), thing("bed_wood"));
        let door = o.offset(2, 4);
        for y in 0..5 {
            for x in 0..5 {
                let p = o.offset(x, y);
                let edge = x == 0 || y == 0 || x == 4 || y == 4;
                if edge && p != door {
                    s.push(Command::Build { thing: w, a: p, b: p });
                }
            }
        }
        s.push(Command::Build { thing: d, a: door, b: door });
        s.push(Command::Build { thing: b, a: o.offset(2, 2), b: o.offset(2, 2) });
    }

    let mut r = Report { seed, min_hp: 1.0, min_food: 1.0, ..Default::default() };
    let mut seen = 0;
    for _ in 0..days * TICKS_PER_DAY {
        s.step();
        let w = &s.world;
        let day = w.tick as f64 / TICKS_PER_DAY as f64;
        if w.tick.is_multiple_of(60) {
            for e in w.colonists() {
                let p = w.ecs.get::<&Pawn>(e).unwrap();
                let max = defs.creature(p.def).max_hp as f64;
                r.min_hp = r.min_hp.min(p.hp as f64 / max);
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
                MsgKind::Bad if m.text.ends_with("has died.") => r.deaths.push((day, m.text.clone())),
                MsgKind::Good => r.goods += 1,
                _ => {}
            }
        }
        seen = w.messages.len();
        if w.colony_lost {
            break;
        }
    }
    let w = &s.world;
    r.colonists_end = w.colonists().count();
    r.survived = r.colonists_end > 0;
    r.wealth = w.wealth;
    r
}

fn main() {
    let seeds = arg("--seeds", 20);
    let days = arg("--days", 5);
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let reports: Vec<Report> = (1..=seeds).map(|seed| play(&mods, seed, days)).collect();

    println!("seed  alive  end  min_hp  min_food  starved_h  hut_day  threats  good  wealth  deaths");
    for r in &reports {
        let starved_h = r.starving_ticks as f64 / TICKS_PER_DAY as f64 * 24.0;
        let hut = r.hut_done_day.map_or("-".into(), |d| format!("{d:.2}"));
        let deaths: Vec<String> = r.deaths.iter().map(|(d, t)| format!("d{d:.1} {t}")).collect();
        println!(
            "{:>4}  {:>5}  {:>3}  {:>6.2}  {:>8.2}  {:>9.1}  {:>7}  {:>7}  {:>4}  {:>6.0}  {}",
            r.seed,
            if r.survived { "yes" } else { "NO" },
            r.colonists_end,
            r.min_hp,
            r.min_food,
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
    println!("hut not done by day 1:    {no_hut}/{n}");
    if !first_threat.is_empty() {
        let mean = first_threat.iter().sum::<f64>() / first_threat.len() as f64;
        let min = first_threat.iter().cloned().fold(f64::MAX, f64::min);
        println!("first threat day:         mean {mean:.2}, earliest {min:.2} ({} runs had one)", first_threat.len());
    }
    let threat_kinds: Vec<&str> = reports.iter().flat_map(|r| r.threats.iter().map(|t| t.1.as_str())).collect();
    for t in threat_kinds.iter().take(12) {
        println!("  {t}");
    }
}
