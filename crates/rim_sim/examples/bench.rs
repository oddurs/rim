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
//! Wall-clock ticks on a busy machine are noisy; the path counts are not.
//!
//! Flags: --seed N, --size N, --colonists N, --pawns N, --days F,
//! --designate-all (every cell of the map designated), --haul (a stockpile
//! and 300 loose stacks instead of other work), --snow CM (a hard, dry
//! frost with CM of snow over the whole surface: `--snow 0` is the same
//! frost with none, to see what snow does to paths), --winter (a thaw:
//! 20 cm of snow over ground wet enough for mud, just above freezing, and
//! the hauling case: what made a708c037's 50-127 ms ticks), --check.
//!
//! `--scale pawns` or `--scale map` plays a ladder instead: pawns 50, 200,
//! 800 and 3,200 on a 500x500 map, or maps of 128, 250, 500 and 1,000 cells
//! a side with 200 pawns, each rung in a process of its own. It prints each
//! system's mean cost per tick at every rung and its growth exponent, the
//! slope of cost against size on a log-log fit; one above 1.15 grows faster
//! than linear and is flagged by name. `--report FILE` writes one run's
//! numbers as JSON, which is how the ladder reads its rungs.
//!
//! `--check` holds the run to `budgets.toml`: the `sim.base` scenario, or
//! `sim.winter` with `--winter`, each cap times the runner class's slack on
//! CI (rim_sim::budgets).

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

/// Two levels dug out under the colony (DESIGN.md §6d): a room on each,
/// stairs down to each, and the rock around both marked for mining, so the
/// work and the hauling cross levels.
fn dig_levels(s: &mut Sim, c: IVec) {
    let defs = s.world.defs.clone();
    let stairs = defs.thing_id("stairs").expect("stairs");
    let mine = defs.lookup("designation", "core:mine").expect("mine");
    let mut top = (0..20)
        .flat_map(|r| (-r..=r).map(move |d| c.offset(r, d)))
        .find(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none())
        .expect("open ground for stairs");
    for z in [-1, -2] {
        let centre = IVec::at(top.x, top.y, z);
        for y in -7..=7 {
            for x in -7..=7 {
                let p = centre.offset(x, y);
                if let Some(leaves) = s.world.solid_at(p).and_then(|r| r.leaves_r) {
                    s.world.map.set_terrain(p, leaves, defs.terrain[leaves as usize].path_cost);
                }
            }
        }
        let e = s.world.spawn_fixture_of(stairs, top, false, None).expect("stairs");
        s.world.open_portal(e);
        s.push(rim_sim::Command::Designate { designation: mine, a: centre.offset(-12, -12), b: centre.offset(12, 12) });
        top = centre.offset(5, 5);
    }
}

fn main() {
    if let Some(axis) = std::env::args().skip_while(|a| a != "--scale").nth(1) {
        return scale(&axis);
    }
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

    let winter = flag("--winter");
    // --haul is the hauling case (cairn 8d551753): a 40x40 stockpile, 300
    // loose stacks of wood and stone, and no other work.
    if flag("--haul") || winter {
        let items: Vec<_> = ["wood", "stone"].iter().filter_map(|id| defs.thing_id(id)).collect();
        let z = c.offset(10, -20);
        s.push(Command::Stockpile { a: z, b: z.offset(39, 39), zone: None });
        let (mut placed, mut tries) = (0, 0);
        while placed < 300 && tries < 100_000 {
            tries += 1;
            let p = c.offset(rng.range(-60, 60), rng.range(-60, 60));
            let d = items[placed % items.len()];
            if s.world.room_for(d, None, p) >= 75 && s.world.zones.at(&s.world.map, p).is_none() {
                s.world.place_item(d, p, 30);
                placed += 1;
            }
        }
    } else {
        plan_work(&mut s, &defs, c, size);
    }
    if flag("--levels") {
        dig_levels(&mut s, c);
    }

    // --winter: a thaw, 1 °C and dry above, 20 cm of snow over ground wet
    // enough for mud (95%, where mud starts costing): both slow a path.
    if winter {
        for (id, v) in [("temperature", 1.0), ("precipitation", 0.0)] {
            if let Some(f) = defs.lookup("field", id) {
                s.world.fields.set_ambient(f as usize, Some(v));
            }
        }
        let snow = defs.lookup("field", "weather:snow").expect("--winter needs the weather plugin") as usize;
        let wet = defs.lookup("field", "weather:wetness").expect("--winter needs the weather plugin") as usize;
        for i in 0..s.world.map.plane() {
            let p = s.world.map.pos(i);
            s.world.fields.set_stock(&defs, &s.world.map, snow, p, 20.0, false);
            s.world.fields.set_stock(&defs, &s.world.map, wet, p, 95.0, false);
        }
    }

    // --snow: frozen and dry, so the snow lies as put and nothing else
    // differs between `--snow 0` and a snowy run.
    let snow_cm: f64 = arg("--snow", -1.0);
    if snow_cm >= 0.0 {
        for (id, v) in [("temperature", -8.0), ("precipitation", 0.0)] {
            if let Some(f) = defs.lookup("field", id) {
                s.world.fields.set_ambient(f as usize, Some(v));
            }
        }
        let snow = defs.lookup("field", "weather:snow").expect("--snow needs the weather plugin") as usize;
        for i in 0..s.world.map.plane() {
            let p = s.world.map.pos(i);
            s.world.fields.set_stock(&defs, &s.world.map, snow, p, snow_cm, false);
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
    // The most path nodes one tick expanded: work, not time, so the same on
    // every machine.
    let mut max_nodes = 0u64;
    for _ in 0..ticks {
        let before = (s.world.pf.searches, s.world.pf.expanded, s.world.pf.micros, s.profile.totals.clone());
        let t = Instant::now();
        s.step();
        let ms = t.elapsed().as_secs_f64() * 1e3;
        times.push(ms);
        max_nodes = max_nodes.max(s.world.pf.expanded - before.1);
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
        "tick: mean {mean:.3} ms · p50 {:.3} · p99 {:.3} · max {:.3}",
        pct(0.5),
        pct(0.99),
        sorted[sorted.len() - 1]
    );
    let machine = rim_sim::budgets::machine();
    println!("machine: {machine}");
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
    println!("most path nodes in a tick: {max_nodes}");
    if let Some(path) = std::env::args().skip_while(|a| a != "--report").nth(1) {
        let per_tick = |us: f64| us / 1000.0 / ticks as f64;
        let mut costs: serde_json::Map<String, serde_json::Value> =
            s.profile.totals.iter().filter(|t| t.0 != "tick").map(|t| (t.0.clone(), per_tick(t.1).into())).collect();
        costs.insert("paths".into(), per_tick(pf.micros - pf0.3).into());
        let report = serde_json::json!({
            "size": size,
            "pawns": w.pawns.len(),
            "ticks": ticks,
            "mean_ms": mean,
            "p99_ms": pct(0.99),
            "max_ms": sorted[sorted.len() - 1],
            "max_nodes": max_nodes,
            "systems": costs,
            "machine": machine,
        });
        std::fs::write(&path, serde_json::to_string_pretty(&report).unwrap()).unwrap_or_else(|e| panic!("{path}: {e}"));
    }
    println!("worst ticks:");
    for (tick, ms, n, nodes, path_ms, sys, sys_ms) in &worst {
        println!(
            "  tick {tick:>7}  {ms:>7.3} ms   {n:>3} searches, {nodes:>6} nodes, {path_ms:>7.3} ms   most in {sys} ({sys_ms:.3} ms)"
        );
    }

    if flag("--check") {
        let budgets = rim_sim::budgets::Budgets::repo().unwrap_or_else(|e| panic!("{e}"));
        let slack = budgets.slack(&machine, std::env::var_os("CI").is_some());
        let scenario = if winter { "winter" } else { "base" };
        let pf = &s.world.pf;
        let per_search = (pf.expanded - pf0.1) as f64 / (pf.searches - pf0.0).max(1) as f64;
        let measured = [
            ("mean_ms", mean),
            ("p99_ms", pct(0.99)),
            ("max_ms", sorted[sorted.len() - 1]),
            ("max_nodes", max_nodes as f64),
            ("nodes_per_search", per_search),
        ];
        let over =
            rim_sim::budgets::Budgets::over(&budgets.sim, scenario, &measured, slack).unwrap_or_else(|e| panic!("{e}"));
        if !over.is_empty() {
            for o in &over {
                eprintln!("bench: sim.{o}");
            }
            std::process::exit(1);
        }
        println!("bench: sim.{scenario} within budgets.toml (slack {slack})");
    }
}

/// Work: chop and mine a large area (or the whole map), plan buildings.
fn plan_work(s: &mut Sim, defs: &rim_sim::defs::DefDb, c: IVec, size: i32) {
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
            s.push(Command::Build { thing: wall, stuff: wood, a: o, b: o.offset(5, 0), facing: 0 });
            s.push(Command::Build { thing: wall, stuff: wood, a: o.offset(0, 4), b: o.offset(5, 4), facing: 0 });
            s.push(Command::Build { thing: bed, stuff: wood, a: o.offset(2, 2), b: o.offset(2, 2), facing: 0 });
        }
    }
}

/// The ladder: each rung is this bench in a process of its own, reporting to
/// a file; then each system's cost against size, and its growth exponent.
fn scale(axis: &str) {
    let days: f64 = arg("--days", 0.1);
    let seed: u64 = arg("--seed", 1);
    let rungs: Vec<(i32, usize)> = match axis {
        "pawns" => [50, 200, 800, 3200].map(|p| (500, p)).to_vec(),
        "map" => [128, 250, 500, 1000].map(|m| (m, 200)).to_vec(),
        _ => {
            eprintln!("bench: --scale takes pawns or map");
            std::process::exit(2);
        }
    };
    let exe = std::env::current_exe().expect("the bench's own path");
    let dir = std::env::temp_dir().join(format!("rim-scale-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut reports: Vec<serde_json::Value> = Vec::new();
    for &(size, pawns) in &rungs {
        let out = dir.join(format!("{size}-{pawns}.json"));
        eprintln!("bench: rung {size}x{size}, {pawns} pawns");
        let status = std::process::Command::new(&exe)
            .args(["--seed", &seed.to_string(), "--size", &size.to_string(), "--pawns", &pawns.to_string()])
            .args(["--days", &days.to_string(), "--report", &out.to_string_lossy()])
            .stdout(std::process::Stdio::null())
            .status()
            .expect("run a rung");
        if !status.success() {
            eprintln!("bench: the rung {size}x{size} with {pawns} pawns failed");
            std::process::exit(1);
        }
        reports.push(serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap());
    }
    let _ = std::fs::remove_dir_all(&dir);
    // The size a rung is measured by: pawns on the pawn ladder, cells on the
    // map ladder.
    let x: Vec<f64> =
        rungs.iter().map(|&(size, pawns)| if axis == "pawns" { pawns as f64 } else { (size as f64).powi(2) }).collect();
    let mut names: Vec<String> =
        reports.iter().flat_map(|r| r["systems"].as_object().unwrap().keys().cloned().collect::<Vec<_>>()).collect();
    names.sort();
    names.dedup();
    let cost = |r: &serde_json::Value, name: &str| r["systems"][name].as_f64().unwrap_or(0.0);
    let mut rows: Vec<(String, Vec<f64>, Option<f64>)> = names
        .iter()
        .map(|n| {
            let ys: Vec<f64> = reports.iter().map(|r| cost(r, n)).collect();
            (n.clone(), ys.clone(), exponent(&x, &ys))
        })
        .collect();
    let ticks: Vec<f64> = reports.iter().map(|r| r["mean_ms"].as_f64().unwrap()).collect();
    rows.push(("(tick)".into(), ticks.clone(), exponent(&x, &ticks)));
    rows.sort_by(|a, b| b.2.unwrap_or(0.0).total_cmp(&a.2.unwrap_or(0.0)));
    let unit = if axis == "pawns" { "pawns" } else { "cells" };
    println!("scale: {axis}, {days} days a rung, seed {seed}, {}", reports[0]["machine"].as_str().unwrap_or(""));
    let head: Vec<String> = x.iter().map(|v| format!("{v:>10.0}")).collect();
    println!("{:<18} {} {:>9}", format!("ms/tick at {unit}"), head.join(" "), "exponent");
    for (name, ys, e) in &rows {
        let cells: Vec<String> = ys.iter().map(|v| format!("{v:>10.4}")).collect();
        let (exp, flag) = match e {
            Some(e) => (format!("{e:>9.2}"), if *e > SUPERLINEAR { "  faster than linear" } else { "" }),
            None => (format!("{:>9}", "-"), ""),
        };
        println!("{name:<18} {} {exp}{flag}", cells.join(" "));
    }
    println!(
        "exponent: the slope of ms/tick against {unit} on a log-log fit over the rungs; 1 is linear, above {SUPERLINEAR} is flagged"
    );
}

/// Where a growth exponent counts as faster than linear: some slack over 1
/// for noise in times this small.
const SUPERLINEAR: f64 = 1.15;

/// The least-squares slope of ln(y) against ln(x), over the points where y
/// is measurable; None when fewer than three are.
fn exponent(x: &[f64], y: &[f64]) -> Option<f64> {
    let pts: Vec<(f64, f64)> = x.iter().zip(y).filter(|(_, &y)| y > 1e-6).map(|(&x, &y)| (x.ln(), y.ln())).collect();
    if pts.len() < 3 {
        return None;
    }
    let n = pts.len() as f64;
    let (mx, my) = (pts.iter().map(|p| p.0).sum::<f64>() / n, pts.iter().map(|p| p.1).sum::<f64>() / n);
    let num: f64 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    let den: f64 = pts.iter().map(|p| (p.0 - mx).powi(2)).sum();
    Some(num / den)
}
