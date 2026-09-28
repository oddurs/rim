//! The climate report: run a year headless and print each day's weather.
//!
//!   cargo run --release -p rim_sim --example year -- --seed 7 [--days 60] [--core]
//!
//! `--core` loads core alone (no weather plugin). Prints one row per day:
//! season, temperature (min / mean / max), hours of precipitation (and how
//! much fell as snow), cloud, wind, the ground's wetness (mean) and snow
//! (deepest) on an open grass cell, and the weather spells that started.
//! Ends with how often each weather type came up.

use rim_sim::data::Data;
use rim_sim::{Sim, TICKS_PER_DAY};
use std::collections::BTreeMap;
use std::path::Path;

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn main() {
    let seed: u64 = arg("--seed").and_then(|v| v.parse().ok()).unwrap_or(7);
    let days: u64 = arg("--days").and_then(|v| v.parse().ok()).unwrap_or(60);
    let core = std::env::args().any(|a| a == "--core");
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let mut s = Sim::with_mods(&mods, seed, &|m| !core || m == "core").expect("mods load");
    let f = |s: &Sim, id: &str| s.world.defs.lookup("field", id).map(|i| i as usize);
    let (temp, precip, cloud, wind) =
        (f(&s, "temperature").unwrap(), f(&s, "precipitation"), f(&s, "cloud"), f(&s, "wind"));
    let (wet, snow) = (f(&s, "weather:wetness"), f(&s, "weather:snow"));
    // Open grass away from the colony, where the weather lands unhindered.
    let ground = {
        let grass = s.world.defs.lookup("terrain", "grass").unwrap();
        let (m, home) = (&s.world.map, s.world.colony_center().unwrap());
        (0..m.plane())
            .map(|i| m.pos(i))
            .find(|&p| p.chebyshev(home) > 30 && m.terrain[m.idx(p)] == grass && m.fixture_at(p).is_none())
    };

    println!("day  season    temp min/mean/max   rain h  snow h  cloud  wind  wet %  snow cm  weather");
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    let mut last_current = String::new();
    let step = TICKS_PER_DAY / 96; // every 15 minutes
    for day in 0..days {
        let (mut lo, mut hi, mut sum) = (f64::MAX, f64::MIN, 0.0);
        let (mut rain_h, mut snow_h, mut cloud_sum, mut wind_sum) = (0.0, 0.0, 0.0, 0.0);
        let (mut wet_sum, mut snow_max) = (0.0, 0.0f64);
        let mut started: Vec<String> = Vec::new();
        let season = s.world.season().to_string();
        for _ in 0..96 {
            for _ in 0..step {
                s.step();
            }
            let a = |i: Option<usize>| i.map_or(0.0, |i| s.world.fields.ambient(i));
            let t = s.world.fields.ambient(temp);
            lo = lo.min(t);
            hi = hi.max(t);
            sum += t;
            if a(precip) > 0.2 {
                if t <= 0.0 {
                    snow_h += 0.25;
                } else {
                    rain_h += 0.25;
                }
            }
            cloud_sum += a(cloud);
            wind_sum += a(wind);
            let at = |i: Option<usize>| match (i, ground) {
                (Some(i), Some(p)) => s.world.fields.value(&s.world.defs, &s.world.map, i, p),
                _ => 0.0,
            };
            wet_sum += at(wet);
            snow_max = snow_max.max(at(snow));
            let current = s
                .world
                .data
                .get("weather:forecast")
                .and_then(|q| if let Data::Table(t) = q { t.values().next().cloned() } else { None })
                .and_then(|e| e.get("id").cloned())
                .and_then(|id| if let Data::Str(s) = id { Some(s) } else { None })
                .unwrap_or_default();
            if !current.is_empty() && current != last_current {
                *counts.entry(current.clone()).or_default() += 1;
                started.push(current.clone());
                last_current = current;
            }
        }
        println!(
            "{:>3}  {:<8} {:>5.1} {:>5.1} {:>5.1}      {:>5.1}   {:>5.1}   {:>4.0}  {:>4.1}  {:>5.0}  {:>7.1}  {}",
            day + 1,
            season,
            lo,
            sum / 96.0,
            hi,
            rain_h,
            snow_h,
            cloud_sum / 96.0,
            wind_sum / 96.0,
            wet_sum / 96.0,
            snow_max,
            started.join(", ")
        );
    }
    let total: u32 = counts.values().sum();
    println!("\nweather spells ({total}):");
    for (k, v) in &counts {
        println!("  {k:<8} {v:>4}  {:>5.1}%", *v as f64 * 100.0 / total.max(1) as f64);
    }
    println!("state hash {:016x}", s.world.state_hash());
}
