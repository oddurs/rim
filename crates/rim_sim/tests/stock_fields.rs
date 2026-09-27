//! Stock fields: a value stored per cell that changes by its rate terms,
//! each cell worked out once a period (DESIGN.md §4c).

mod common;

use rim_sim::data::Data;
use rim_sim::field::Clock;
use rim_sim::snapshot::Snapshot;
use rim_sim::terms::{to_q, Q};
use rim_sim::world::World;
use rim_sim::{IVec, Sim, TICKS_PER_DAY};
use std::sync::Arc;

/// Wetness settles toward the water table at half the gap an hour; a
/// counter goes up by one an hour everywhere, to count the updates.
const DAMP: &str = r##"
[[field]]
id = "wet"
label = "wetness"
kind = "stock"
range = [0.0, 1.0]
period_minutes = 60
color_low = "#000000"
color_high = "#ffffff"

[field.init.dry]
of = [0.1]

[field.base.table]
of = [{ terrain = "water_table" }]

[field.rate.settle]
scale = -0.5
of = [{ input = "above_base" }]

[[field]]
id = "count"
label = "count"
kind = "stock"
range = [0.0, 100000.0]
period_minutes = 60
levels = "all"
overlay = false
color_low = "#000000"
color_high = "#ffffff"

[field.rate.one]
of = [1.0]

[[thing]]
id = "sprinkler"
label = "sprinkler"
color = "#6fa8dc"
category = "building"
hp = 50
emit = [{ field = "wet", amount = 2.0, radius = 2 }]
"##;

const PROBE: &str = r#"
local M = {}
function M.soak(x, y)
    rim.field_set("wet", x, y, 0.25)
    return rim.field_add("wet", x, y, 0.5)
end
function M.not_stock(x, y)
    return rim.field_add("core:temperature", x, y, 1)
end
return M
"#;

fn sim(name: &str) -> (Sim, std::path::PathBuf) {
    let files: &[(&str, &str)] = &[("defs/damp.toml", DAMP), ("scripts/probe.luau", PROBE)];
    let dir = common::test_mods(name, &["core"], &[("damp", files)]);
    (Sim::new(&dir, 5).expect("mods load"), dir)
}

fn field(s: &Sim, id: &str) -> usize {
    field_of(&s.world.defs, id)
}

fn field_of(defs: &rim_sim::defs::DefDb, id: &str) -> usize {
    defs.lookup("field", id).unwrap() as usize
}

fn value(s: &Sim, id: &str, p: IVec) -> f64 {
    s.world.fields.value(&s.world.defs, &s.world.map, field(s, id), p)
}

fn period(s: &Sim) -> u64 {
    s.world.defs.fields[field(s, "damp:wet")].period
}

fn grass(s: &mut Sim, p: IVec) {
    let t = s.world.defs.lookup("terrain", "grass").unwrap();
    s.world.map.set_terrain(p, t, 100);
}

#[test]
fn a_stock_field_rises_falls_and_settles_to_its_base() {
    let (mut s, _) = sim("stock-settle");
    let (wet, at) = (field(&s, "damp:wet"), s.world.colony_center().unwrap().offset(9, 9));
    let (up, down) = (at, at.offset(1, 0));
    grass(&mut s, up);
    grass(&mut s, down);
    s.step();
    // It starts from its init terms, then heads for grass's water table, 0.4.
    let defs = s.world.defs.clone();
    s.world.fields.set_stock(&defs, &s.world.map, wet, up, 0.0, false);
    s.world.fields.set_stock(&defs, &s.world.map, wet, down, 1.0, false);
    let (mut last_up, mut last_down) = (0.0, 1.0);
    for _ in 0..12 {
        for _ in 0..period(&s) {
            s.step();
        }
        let (u, d) = (value(&s, "damp:wet", up), value(&s, "damp:wet", down));
        assert!(u >= last_up && u <= 0.4 && d <= last_down && d >= 0.4, "up {u}, down {d}");
        (last_up, last_down) = (u, d);
    }
    assert!((last_up - 0.4).abs() < 0.01 && (last_down - 0.4).abs() < 0.01, "{last_up} {last_down}");
}

/// Every cell once a period, over the whole period, on any map: a rate of
/// one an hour adds one period's hours to each cell, exactly.
#[test]
fn every_cell_updates_once_a_period_whatever_the_map_size() {
    let (s, _) = sim("stock-once");
    let defs = s.world.defs.clone();
    let count = field(&s, "damp:count");
    let p = defs.fields[count].period;
    let per_period = to_q(1.0) * p as i64 * 24 / TICKS_PER_DAY as i64;
    for (w, h, below, start) in [(1, 1, 0, 0), (7, 3, 0, 5), (64, 64, 2, 1234), (250, 250, 0, 99)] {
        let mut world = World::with_levels(Arc::clone(&defs), w, h, below, 0, 1);
        for periods in 1..=2 {
            for t in 0..p {
                let clock = Clock { tick: start + (periods - 1) * p + t, year: 0, hour: 0, seed: 1 };
                world.fields.step_stock(&defs, &world.map, clock);
            }
            let cells = &world.fields.layers[count].stock;
            assert_eq!(cells.len(), world.map.cells());
            // Wetness is kept on the surface only, and reads 0 below it.
            let wet = field_of(&defs, "damp:wet");
            assert_eq!(world.fields.layers[wet].stock.len(), world.map.plane());
            if below > 0 {
                assert_eq!(world.fields.value(&defs, &world.map, wet, IVec::at(0, 0, -1)), 0.0);
            }
            assert!(
                cells.iter().all(|&v| v as i64 == per_period * periods as i64),
                "{w}×{h}×{}: after {periods} periods some cell isn't {}",
                below + 1,
                per_period * periods as i64
            );
        }
    }
}

#[test]
fn emitters_and_scripts_add_to_a_stock_field() {
    let (mut s, _) = sim("stock-add");
    let at = s.world.colony_center().unwrap().offset(8, -8);
    for y in -3..=3 {
        for x in -3..=3 {
            if let Some(f) = s.world.map.fixture_at(at.offset(x, y)) {
                s.world.despawn_thing(f);
            }
            grass(&mut s, at.offset(x, y));
        }
    }
    let sprinkler = s.world.defs.thing_id("damp:sprinkler").unwrap();
    s.world.spawn_fixture(sprinkler, at, false).expect("a sprinkler");
    for _ in 0..3 * period(&s) {
        s.step();
    }
    // Two an hour at the source outpaces the pull back to 0.4: soaked. Out
    // of reach it only settles.
    assert_eq!(value(&s, "damp:wet", at), 1.0);
    assert!(value(&s, "damp:wet", at.offset(2, 0)) > value(&s, "damp:wet", at.offset(3, 0)));
    assert!(value(&s, "damp:wet", at.offset(3, 0)) <= 0.4);

    let dry = at.offset(-3, 3);
    let args = [Some(Data::Int(dry.x as i64)), Some(Data::Int(dry.y as i64))];
    let got = s.scripts.call_export(&mut s.world, "@damp/scripts/probe", "soak", &args).unwrap();
    assert_eq!(got, Some(Data::Num(0.75)));
    assert_eq!(value(&s, "damp:wet", dry), 0.75);
    let e = s.scripts.call_export(&mut s.world, "@damp/scripts/probe", "not_stock", &args).unwrap_err();
    assert!(e.contains("isn't a stock field"), "{e}");
}

#[test]
fn a_load_keeps_stock_values_and_the_hash_sees_them() {
    let (mut s, dir) = sim("stock-save");
    for _ in 0..2 * period(&s) {
        s.step();
    }
    let (wet, at) = (field(&s, "damp:wet"), s.world.colony_center().unwrap().offset(5, 5));
    let defs = s.world.defs.clone();
    let before = s.world.state_hash();
    s.world.fields.set_stock(&defs, &s.world.map, wet, at, 0.9, false);
    assert_ne!(s.world.state_hash(), before, "the hash covers stock values");
    let loaded = Snapshot::capture(&s).restore(&dir, &|_| true).expect("loads");
    assert_eq!(loaded.world.fields.layers[wet].stock, s.world.fields.layers[wet].stock);
    assert_eq!(loaded.world.state_hash(), s.world.state_hash());
}

/// A year of a field that reads the hour, the season, noise and itself:
/// the same seed gives the same values, and another seed doesn't.
#[test]
fn the_same_seed_gives_the_same_values_after_a_year() {
    const SWING: &str = r##"
[[field]]
id = "swing"
label = "swing"
kind = "stock"
range = [-50.0, 50.0]
period_minutes = 180
color_low = "#000000"
color_high = "#ffffff"

[field.rate.day]
of = [{ input = "hour", curve = [[0, -1.0], [12, 1.0], [24, -1.0]] }]
[field.rate.season]
of = [{ input = "year", curve = [[0, 0.5], [0.5, -0.5], [1, 0.5]] }]
[field.rate.gusts]
of = [{ noise = "swing", hours = 5 }]
[field.rate.drag]
scale = -0.05
of = [{ input = "self" }]
"##;
    let dir = common::test_mods("stock-year", &["core"], &[("swing", &[("defs/swing.toml", SWING)])]);
    let defs = Sim::new(&dir, 1).expect("mods load").world.defs.clone();
    let f = defs.lookup("field", "swing:swing").unwrap() as usize;
    let year = defs.calendar.year_days as u64 * TICKS_PER_DAY;
    let run = |seed: u64| {
        let mut w = World::new(Arc::clone(&defs), 40, 40, seed);
        for tick in 0..year {
            let clock = Clock {
                tick,
                year: (tick % year) as i64 * Q / year as i64,
                hour: (tick % TICKS_PER_DAY) as i64 * 24 * Q / TICKS_PER_DAY as i64,
                seed,
            };
            w.fields.step_stock(&defs, &w.map, clock);
        }
        w.fields.layers[f].stock.clone()
    };
    let a = run(3);
    assert!(a.iter().any(|&v| v != 0), "it moved");
    assert_eq!(a, run(3));
    assert_ne!(a, run(4));
}

/// 250×250 with two stock fields: the mean cost of a tick's slices.
#[test]
fn two_stock_fields_on_a_big_map_are_cheap() {
    let (s, _) = sim("stock-cost");
    let defs = s.world.defs.clone();
    let mut w = World::new(Arc::clone(&defs), 250, 250, 1);
    let ticks = 4 * period(&s);
    let mut fastest = f64::MAX;
    for round in 0..3 {
        let t0 = std::time::Instant::now();
        for t in 0..ticks {
            let clock = Clock { tick: round * ticks + t, year: 0, hour: 0, seed: 1 };
            w.fields.step_stock(&defs, &w.map, clock);
        }
        fastest = fastest.min(t0.elapsed().as_secs_f64() * 1e3 / ticks as f64);
    }
    println!("two stock fields, 250×250: {fastest:.4} ms a tick");
    let slack = if std::env::var_os("CI").is_some() { 6.0 } else { 1.0 };
    assert!(fastest <= 0.02 * slack, "{fastest:.4} ms a tick");
}
