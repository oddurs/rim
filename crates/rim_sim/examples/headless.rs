//! Run the simulation with no window.
//!
//!   cargo run --release -p rim_sim --example headless -- --days 5 --seed 42 [--core]

use rim_sim::world::{Faction, Pawn};
use rim_sim::{Sim, TICKS_PER_DAY};
use std::path::Path;
use std::time::Instant;

fn arg(name: &str, default: u64) -> u64 {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn main() {
    let days = arg("--days", 5);
    let seed = arg("--seed", 42);
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let t = Instant::now();
    // --core: core alone, proving the base game stands without plugins.
    let core = std::env::args().any(|a| a == "--core");
    let mut sim = Sim::with_mods(&mods, seed, &|m| !core || m == "core").unwrap_or_else(|e| panic!("load failed: {e}"));
    println!("loaded {} mods in {:?}", sim.mods.len(), t.elapsed());
    for w in &sim.warnings {
        println!("  warning: {w}");
    }

    let mut times = Vec::with_capacity((days * TICKS_PER_DAY) as usize);
    let mut seen = 0;
    for _ in 0..days * TICKS_PER_DAY {
        let t = Instant::now();
        sim.step();
        times.push(t.elapsed().as_secs_f64() * 1e3);
        for m in &sim.world.messages[seen..] {
            let hour = ((m.tick + TICKS_PER_DAY / 4) % TICKS_PER_DAY) as f64 / TICKS_PER_DAY as f64 * 24.0;
            println!("day {:>2} {:>5.1}h  {}", m.tick / TICKS_PER_DAY, hour, m.text);
        }
        seen = sim.world.messages.len();
    }

    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    let p99 = times[times.len() * 99 / 100];
    let w = &sim.world;
    let count = |f: Faction| w.pawns.iter().filter(|&&e| w.ecs.get::<&Pawn>(e).is_ok_and(|p| p.faction == f)).count();
    println!(
        "\n{} days · colonists {} · hostiles {} · wild {} · wealth {:.0}",
        days,
        count(Faction::Player),
        count(Faction::Hostile),
        count(Faction::Wild),
        w.wealth
    );
    println!("tick: mean {mean:.3} ms · p99 {p99:.3} ms · max {:.3} ms", times.last().unwrap());
    println!("paths: {} searches, {} nodes expanded", w.pf.searches, w.pf.expanded);
    println!("state hash {:016x}", w.state_hash());
}
