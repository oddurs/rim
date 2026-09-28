//! Fire (mods/fire): the wind carries it, rain and wet ground put it out,
//! dry lightning starts it, and colonists beat it out near home.

mod common;

use rim_sim::data::Data;
use rim_sim::world::Pawn;
use rim_sim::{IVec, Sim, TICKS_PER_DAY};
use std::path::Path;

const FIRE: &str = "@fire/scripts/fire";

fn sim(seed: u64) -> Sim {
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let mut s = Sim::with_mods(&mods, seed, &|m| matches!(m, "core" | "weather" | "fire")).expect("mods load");
    s.step();
    s
}

fn pin(s: &mut Sim, id: &str, v: f64) {
    let f = s.world.defs.lookup("field", id).unwrap() as usize;
    s.world.fields.set_ambient(f, Some(v));
}

/// Still, dry air: `wind` m/s blowing east.
fn weather(s: &mut Sim, wind: f64, rain: f64) {
    pin(s, "core:wind", wind);
    pin(s, "core:wind_dir", 0.0);
    pin(s, "core:precipitation", rain);
}

/// A clear square of grass `r` cells out from `c`, its ground `wet` %.
fn meadow(s: &mut Sim, c: IVec, r: i32, wet: f64) {
    let grass = s.world.defs.lookup("terrain", "grass").unwrap();
    let wetness = s.world.defs.lookup("field", "weather:wetness").unwrap() as usize;
    let defs = s.world.defs.clone();
    for y in -r..=r {
        for x in -r..=r {
            let p = c.offset(x, y);
            if !s.world.map.inb(p) {
                continue;
            }
            for e in [s.world.map.fixture_at(p), s.world.map.item_at(p), s.world.map.floor_at(p)].into_iter().flatten()
            {
                s.world.despawn_thing(e);
            }
            s.world.map.set_terrain(p, grass, 100);
            s.world.fields.set_stock(&defs, &s.world.map, wetness, p, wet, false);
        }
    }
}

/// A clear spot well away from the colony and the map's edge.
fn spot(s: &Sim, r: i32) -> IVec {
    let m = &s.world.map;
    let home = s.world.colony_center().unwrap();
    let c = IVec::new(m.w / 2, m.h / 2);
    if c.chebyshev(home) > r + 35 {
        c
    } else {
        IVec::new(if home.x > m.w / 2 { r + 2 } else { m.w - r - 3 }, m.h / 2)
    }
}

fn call(s: &mut Sim, f: &str, args: &[Option<Data>]) -> Option<Data> {
    s.scripts.call_export(&mut s.world, FIRE, f, args).unwrap_or_else(|e| panic!("{f}: {e}"))
}

fn at(p: IVec) -> [Option<Data>; 2] {
    [Some(Data::Int(p.x as i64)), Some(Data::Int(p.y as i64))]
}

/// Every cell that has burned or is burning.
fn burnt(s: &mut Sim) -> Vec<IVec> {
    let mut out = Vec::new();
    if let Some(Data::Table(t)) = call(s, "burning", &[]) {
        for b in t.values() {
            let (Some(Data::Int(x)), Some(Data::Int(y))) = (b.get("x"), b.get("y")) else { continue };
            out.push(IVec::new(*x as i32, *y as i32));
        }
    }
    if let Some(Data::Table(t)) = s.world.data.get("fire:scorched") {
        for k in t.keys() {
            let rim_sim::data::Key::Str(k) = k else { continue };
            let (x, y) = k.split_once(',').unwrap();
            out.push(IVec::new(x.parse().unwrap(), y.parse().unwrap()));
        }
    }
    out
}

fn steps(s: &mut Sim, n: usize) {
    for _ in 0..n {
        call(s, "step", &[]);
    }
}

#[test]
fn a_fire_spreads_downwind_faster_than_upwind() {
    let mut s = sim(3);
    let c = spot(&s, 20);
    meadow(&mut s, c, 20, 10.0);
    weather(&mut s, 12.0, 0.0);
    assert_eq!(call(&mut s, "ignite", &at(c)), Some(Data::Bool(true)));
    steps(&mut s, 12);
    let cells = burnt(&mut s);
    let east = cells.iter().filter(|p| p.x > c.x).count();
    let west = cells.iter().filter(|p| p.x < c.x).count();
    println!("after 12 steps: {} cells, {east} east of the start, {west} west", cells.len());
    assert!(east > 3 * west.max(1), "east {east} west {west}");
    let reach = |d: i32| cells.iter().map(|p| (p.x - c.x) * d).max().unwrap_or(0);
    assert!(reach(1) > reach(-1), "it runs further downwind");
}

#[test]
fn rain_puts_a_fire_out_and_wet_ground_does_not_catch() {
    let mut s = sim(4);
    let c = spot(&s, 8);
    meadow(&mut s, c, 8, 10.0);
    weather(&mut s, 6.0, 0.0);
    call(&mut s, "ignite", &at(c));
    steps(&mut s, 3);
    assert!(!burnt(&mut s).is_empty());
    // A downpour: out within a few steps.
    pin(&mut s, "core:precipitation", 6.0);
    steps(&mut s, 3);
    let left = match call(&mut s, "burning", &[]) {
        Some(Data::Table(t)) => t.len(),
        _ => 0,
    };
    assert_eq!(left, 0, "rain puts it out");

    // Soaked ground: a fire lit there burns out where it started.
    let mut s = sim(4);
    meadow(&mut s, c, 8, 75.0);
    weather(&mut s, 6.0, 0.0);
    call(&mut s, "ignite", &at(c));
    steps(&mut s, 20);
    assert_eq!(burnt(&mut s), vec![c], "nothing around it caught");
    // And lightning doesn't catch on it.
    assert_eq!(call(&mut s, "strike", &at(c.offset(3, 3))), Some(Data::Bool(false)));
}

#[test]
fn lightning_starts_a_fire_in_a_dry_storm() {
    let mut s = sim(5);
    let c = spot(&s, 4);
    meadow(&mut s, c, 4, 15.0);
    weather(&mut s, 12.0, 0.0);
    assert_eq!(call(&mut s, "strike", &at(c)), Some(Data::Bool(true)), "a dry strike catches");
    pin(&mut s, "core:precipitation", 7.0);
    assert_eq!(call(&mut s, "strike", &at(c.offset(2, 2))), Some(Data::Bool(false)), "a wet one doesn't");

    // The weather plugin's dry storm, left to strike on its own.
    let mut s = sim(5);
    let id = [Some(Data::Str("fire:dry_storm".into())), Some(Data::Num(24.0))];
    s.scripts.call_export(&mut s.world, "@weather/scripts/weather", "force", &id).expect("forced");
    let started = (0..TICKS_PER_DAY).any(|_| {
        s.step();
        s.world.data.get("fire:burning").is_some_and(|d| matches!(d, Data::Table(t) if !t.is_empty()))
    });
    assert!(started, "a day of dry storm starts a fire");
}

#[test]
fn colonists_beat_out_a_fire_near_home() {
    let mut s = sim(6);
    common::hands(&mut s);
    for e in s.world.colonists().collect::<Vec<_>>() {
        let mut p = s.world.ecs.get::<&mut Pawn>(e).unwrap();
        p.needs.iter_mut().for_each(|n| n.1 = rim_sim::world::NEED_MAX);
    }
    let c = s.world.colony_center().unwrap().offset(6, 0);
    meadow(&mut s, c, 3, 55.0);
    weather(&mut s, 1.0, 0.0);
    call(&mut s, "ignite", &at(c));
    let flames = s.world.map.floor_at(c).expect("flames on the ground");
    let beat = (0..TICKS_PER_DAY / 4).any(|_| {
        s.step();
        s.world.thing(flames).is_none()
    });
    assert!(beat, "a colonist beat it out");
}

#[test]
fn a_burning_map_is_cheap() {
    let mut s = sim(7);
    let c = spot(&s, 20);
    meadow(&mut s, c, 20, 5.0);
    weather(&mut s, 8.0, 0.0);
    for y in -8..=8 {
        for x in -8..=8 {
            call(&mut s, "ignite", &at(c.offset(x, y)));
        }
    }
    let n = match call(&mut s, "burning", &[]) {
        Some(Data::Table(t)) => t.len(),
        _ => 0,
    };
    // The fastest of a few steps: one alone is at the mercy of the machine.
    let ms = (0..5)
        .map(|_| {
            let t0 = std::time::Instant::now();
            call(&mut s, "step", &[]);
            t0.elapsed().as_secs_f64() * 1e3
        })
        .fold(f64::MAX, f64::min);
    println!("one step with {n} cells burning: {ms:.3} ms, {:.4} ms a tick", ms / 250.0);
    let slack = if std::env::var_os("CI").is_some() { 6.0 } else { 3.0 };
    // A Luau step has no work to count from outside, so this is a budget,
    // timed only alone, in CI's budget step (DESIGN.md §8a).
    if std::env::var_os("RIM_BUDGETS").is_some() {
        assert!(ms < 10.0 * slack, "{ms:.3} ms a step");
    }
}
