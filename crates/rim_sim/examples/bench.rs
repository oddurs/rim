//! The benchmark harness: the performance target from DESIGN.md §8.
//!
//!   cargo run --release -p rim_sim --example bench
//!   cargo run --release -p rim_sim --example bench -- --days 0.5 --check
//!
//! A reproducible scenario from a seed: a 250x250 map, 30 colonists and
//! wildlife up to 200 pawns, with work to do (a large area designated for
//! chopping and mining, walls and beds planned). After a warm-up it measures
//! every tick and reports mean, p50, p99 and max, each system's mean cost
//! per tick, what pathfinding cost, and the worst ticks with what they did.
//! Wall-clock ticks on a busy machine are noisy; the path counts are not. `--check` exits non-zero if the mean tick is over budget (2 ms,
//! the time a tick has at 6x speed), with 3x slack on shared CI runners.
//!
//! Flags: --seed N, --size N, --colonists N, --pawns N, --days F,
//! --designate-all (every cell of the map designated), --check.

use rim_sim::world::Faction;
use rim_sim::{Command, IVec, Sim, TICKS_PER_DAY};
use std::path::Path;
use std::time::Instant;

fn arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn flag(name: &str) -> bool {
    std::env::args().any(|a| a == name)
}

/// Budget per tick at 6x speed and 60 fps (DESIGN.md §8), in ms.
const BUDGET_MS: f64 = 2.0;

fn main() {
    let seed: u64 = arg("--seed", 1);
    let size: i32 = arg("--size", 250);
    let colonists: usize = arg("--colonists", 30);
    let pawns: usize = arg("--pawns", 200);
    let days: f64 = arg("--days", 1.0);
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let t = Instant::now();
    let mut s = Sim::build(&mods, seed, &|_| true, size).expect("mods load");
    let load_ms = t.elapsed().as_secs_f64() * 1e3;
    let defs = s.world.defs.clone();
    let c = s.world.colony_center().expect("a colony");

    // Colonists around the start, then wildlife anywhere open until the
    // pawn count is reached: mostly grazers, some predators.
    let human = defs.creature_id("human").expect("human");
    let mut rng = rim_sim::rng::Rng::new(seed ^ 0xBE7C);
    let mut tries = 0;
    while s.world.colonists().count() < colonists && tries < 10_000 {
        tries += 1;
        let p = c.offset(rng.range(-6, 6), rng.range(-6, 6));
        if s.world.map.passable(p) {
            s.world.spawn_pawn(human, Faction::Player, p, None);
        }
    }
    let wild: Vec<_> = ["deer", "hare", "boar", "wolf"].iter().filter_map(|id| defs.creature_id(id)).collect();
    tries = 0;
    while s.world.pawns.len() < pawns && tries < 100_000 {
        tries += 1;
        let p = IVec::new(rng.range(0, size - 1), rng.range(0, size - 1));
        if s.world.map.passable(p) && p.chebyshev(c) > 20 {
            // One predator in eight.
            let def =
                if rng.below(8) == 0 { *wild.last().unwrap() } else { wild[rng.below(wild.len() as u32 - 1) as usize] };
            s.world.spawn_pawn(def, Faction::Wild, p, None);
        }
    }

    // The stone age gates chopping and mining behind tools: an axe and a
    // hammerstone for every colonist, so the work below is done.
    for t in ["primitive:hand_axe", "primitive:hammerstone"].iter().filter_map(|id| defs.thing_id(id)) {
        for col in s.world.colonists().collect::<Vec<_>>() {
            if let Some(at) = s.world.pawn_pos(col) {
                s.world.place_item(t, at, 1);
            }
        }
    }

    // Work: chop and mine a large area (or the whole map), plan buildings.
    let des = |id: &str| defs.lookup("designation", id).expect(id);
    let (a, b) = if flag("--designate-all") {
        (IVec::new(0, 0), IVec::new(size - 1, size - 1))
    } else {
        (c.offset(-40, -40), c.offset(40, 40))
    };
    for d in ["chop", "mine", "harvest"] {
        s.push(Command::Designate { designation: des(d), a, b });
    }
    let wood = defs.thing_id("wood");
    if let (Some(wall), Some(bed)) = (defs.thing_id("wall"), defs.thing_id("bed")) {
        for k in 0..6 {
            let o = c.offset(8 + k * 7, 8);
            s.push(Command::Build { thing: wall, stuff: wood, a: o, b: o.offset(5, 0) });
            s.push(Command::Build { thing: wall, stuff: wood, a: o.offset(0, 4), b: o.offset(5, 4) });
            s.push(Command::Build { thing: bed, stuff: wood, a: o.offset(2, 2), b: o.offset(2, 2) });
        }
    }

    // Warm up (paths, rooms, first jobs), then measure.
    for _ in 0..600 {
        s.step();
    }
    s.profile.reset_totals();
    let ticks = (days * TICKS_PER_DAY as f64) as usize;
    let mut times = Vec::with_capacity(ticks);
    // What each tick did, to name the worst ones: (tick, ms, searches,
    // nodes, path ms, the system that took longest and its ms).
    let mut worst: Vec<(u64, f64, u64, u64, f64, String, f64)> = Vec::new();
    let pf0 = (s.world.pf.searches, s.world.pf.expanded, s.world.pf.failed, s.world.pf.micros);
    for _ in 0..ticks {
        let before = (s.world.pf.searches, s.world.pf.expanded, s.world.pf.micros, s.profile.totals.clone());
        let t = Instant::now();
        s.step();
        let ms = t.elapsed().as_secs_f64() * 1e3;
        times.push(ms);
        if worst.len() < 8 || ms > worst[worst.len() - 1].1 {
            let spent = |name: &str| before.3.iter().find(|b| b.0 == name).map_or(0.0, |b| b.1);
            let (sys, us) = s
                .profile
                .totals
                .iter()
                .filter(|t| t.0 != "tick")
                .map(|t| (t.0.clone(), t.1 - spent(&t.0)))
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap_or_default();
            let pf = &s.world.pf;
            let row = (
                s.world.tick,
                ms,
                pf.searches - before.0,
                pf.expanded - before.1,
                (pf.micros - before.2) / 1e3,
                sys,
                us / 1e3,
            );
            worst.push(row);
            worst.sort_by(|a, b| b.1.total_cmp(&a.1));
            worst.truncate(8);
        }
    }
    let mut sorted = times.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    let pct = |p: f64| sorted[((sorted.len() as f64 * p) as usize).min(sorted.len() - 1)];

    let w = &s.world;
    let count = |f: Faction| {
        w.pawns.iter().filter(|&&e| w.ecs.get::<&rim_sim::world::Pawn>(e).is_ok_and(|p| p.faction == f)).count()
    };
    println!(
        "bench: seed {seed}, {size}x{size}, {} colonists, {} pawns ({} wild, {} hostile) at the end, {ticks} ticks measured, load {load_ms:.0} ms",
        count(Faction::Player),
        w.pawns.len(),
        count(Faction::Wild),
        count(Faction::Hostile)
    );
    println!(
        "tick: mean {mean:.3} ms · p50 {:.3} · p99 {:.3} · max {:.3}   (budget {BUDGET_MS} ms at 6x)",
        pct(0.5),
        pct(0.99),
        sorted[sorted.len() - 1]
    );
    let mut systems: Vec<_> = s.profile.totals.iter().filter(|t| t.0 != "tick").cloned().collect();
    systems.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    println!("per system, mean ms per tick:");
    for (name, total_us, calls) in systems.iter().take(12) {
        println!("  {name:<16} {:>8.4}   ({calls} calls)", total_us / 1000.0 / ticks as f64);
    }

    let pf = &s.world.pf;
    let searches = pf.searches - pf0.0;
    println!(
        "paths: {:.2} searches per tick, {:.0} nodes per search, {:.1}% found nothing, {:.4} ms per tick",
        searches as f64 / ticks as f64,
        (pf.expanded - pf0.1) as f64 / searches.max(1) as f64,
        (pf.failed - pf0.2) as f64 * 100.0 / searches.max(1) as f64,
        (pf.micros - pf0.3) / 1e3 / ticks as f64
    );
    println!("worst ticks:");
    for (tick, ms, n, nodes, path_ms, sys, sys_ms) in &worst {
        println!(
            "  tick {tick:>7}  {ms:>7.3} ms   {n:>3} searches, {nodes:>6} nodes, {path_ms:>7.3} ms   most in {sys} ({sys_ms:.3} ms)"
        );
    }

    if flag("--check") {
        let slack = if std::env::var_os("CI").is_some() { 3.0 } else { 1.0 };
        let limit = BUDGET_MS * slack;
        if mean > limit {
            eprintln!("bench: mean tick {mean:.3} ms is over the budget of {limit:.1} ms");
            std::process::exit(1);
        }
        println!("bench: within budget ({mean:.3} <= {limit:.1} ms)");
    }
}
