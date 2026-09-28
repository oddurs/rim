//! What water costs (DESIGN.md §6d): three levels dug out under a 250 × 250
//! map, joined by holes, breached into a river at the top. Times the water
//! alone, a tick at a time: while it floods, at rest once full, and the
//! rebuild when one more cell is dug.
//!
//!   cargo run --release -p rim_sim --example flood -- [--size 250] [--hall 120]

use rim_sim::{IVec, Sim};
use std::path::Path;
use std::time::Instant;

fn arg(name: &str, default: i32) -> i32 {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn stats(label: &str, v: &mut [f64]) {
    v.sort_by(f64::total_cmp);
    let mean = v.iter().sum::<f64>() / v.len().max(1) as f64;
    let worst = v.last().copied().unwrap_or(0.0);
    println!(
        "{label:<10} {:>7} ticks   mean {mean:.4} ms   p99 {:.4}   worst {worst:.4}",
        v.len(),
        v[(v.len() * 99 / 100).min(v.len() - 1)]
    );
}

fn main() {
    let size = arg("--size", 250);
    let hall = arg("--hall", 120);
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let mut s = Sim::build(&mods, 1, &|m| m == "core", size).expect("core loads");
    s.step();
    let defs = s.world.defs.clone();
    let id = |name: &str| defs.terrain.iter().position(|t| t.id == name).expect(name) as rim_sim::defs::DefId;
    let (air, river) = (id("core:air"), id("core:deep_water"));
    let o = (size - hall) / 2;
    // Three halls, each a hole in the middle of the one above it.
    let mut dug = 0;
    for z in [-1, -2, -3] {
        for y in o..o + hall {
            for x in o..o + hall {
                let p = IVec::at(x, y, z);
                if let Some(leaves) = s.world.solid_at(p).and_then(|r| r.leaves_r) {
                    s.world.map.set_terrain(p, leaves, defs.terrain[leaves as usize].path_cost);
                    dug += 1;
                }
            }
        }
    }
    let mid = o + hall / 2;
    for z in [-1, -2] {
        s.world.map.set_terrain(IVec::at(mid, mid, z), air, 0);
    }
    // Build the basins before the river comes in, so the rebuild isn't
    // counted as flooding.
    let t = Instant::now();
    s.world.update_water();
    println!("{size} x {size}, three halls of {hall} x {hall}: {dug} cells dug");
    println!("first build: {:.3} ms", t.elapsed().as_secs_f64() * 1e3);
    // The breach: a river cell in the top hall's wall. Its tick rebuilds
    // the top level; the ticks after only move water.
    s.world.map.set_terrain(IVec::at(o - 1, mid, -1), river, 0);
    let t = Instant::now();
    s.world.update_water();
    println!("the breach: {:.3} ms (the top level rebuilt)", t.elapsed().as_secs_f64() * 1e3);
    let full = |s: &Sim| [-1, -2, -3].iter().all(|&z| s.world.water.basins(z).iter().all(|b| b.volume >= b.capacity()));
    let mut flooding = Vec::new();
    while !full(&s) && flooding.len() < 2_000_000 {
        let t = Instant::now();
        s.world.update_water();
        flooding.push(t.elapsed().as_secs_f64() * 1e3);
    }
    stats("flooding", &mut flooding);
    let mut rest = Vec::new();
    for _ in 0..20_000 {
        let t = Instant::now();
        s.world.update_water();
        rest.push(t.elapsed().as_secs_f64() * 1e3);
    }
    stats("at rest", &mut rest);
    // Digging on beside the flooded top hall, a cell at a time: each
    // rebuilds its level and carries the water over. Min and median, since
    // a busy machine only ever adds.
    let mut digs = Vec::new();
    for k in 0..30 {
        let p = IVec::at(o + hall, o + k, -1);
        if let Some(leaves) = s.world.solid_at(p).and_then(|r| r.leaves_r) {
            s.world.map.set_terrain(p, leaves, defs.terrain[leaves as usize].path_cost);
        }
        let t = Instant::now();
        s.world.update_water();
        digs.push(t.elapsed().as_secs_f64() * 1e3);
    }
    digs.sort_by(f64::total_cmp);
    println!(
        "a dig:     min {:.3} ms, median {:.3} ms, of {} (its level rebuilt, water carried over)",
        digs[0],
        digs[digs.len() / 2],
        digs.len()
    );
}
