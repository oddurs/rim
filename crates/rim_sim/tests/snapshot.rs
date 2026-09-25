//! A snapshot is a cache of the game (DESIGN.md §7a): loading one and
//! carrying on must be indistinguishable from never having saved.

mod common;

use rim_sim::snapshot::Snapshot;
use rim_sim::{Command, Sim, TICKS_PER_DAY};
use std::path::{Path, PathBuf};

fn mods() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

fn colony(seed: u64) -> Sim {
    let mut sim = Sim::new(&mods(), seed).expect("mods load");
    let chop = sim.world.defs.lookup("designation", "chop").unwrap();
    let wall = sim.world.defs.thing_id("wall").unwrap();
    let door = sim.world.defs.thing_id("door").unwrap();
    let wood = sim.world.defs.thing_id("wood").unwrap();
    let c = sim.world.colony_center().unwrap();
    // A hut, so there are rooms, a door, blueprints and hauling.
    sim.push(Command::Designate { designation: chop, a: c.offset(-12, -12), b: c.offset(12, 12) });
    sim.push(Command::Build { stuff: Some(wood), thing: wall, a: c.offset(2, 2), b: c.offset(6, 2) });
    sim.push(Command::Build { stuff: Some(wood), thing: wall, a: c.offset(2, 6), b: c.offset(6, 6) });
    sim.push(Command::Build { stuff: Some(wood), thing: wall, a: c.offset(2, 3), b: c.offset(2, 5) });
    sim.push(Command::Build { stuff: Some(wood), thing: wall, a: c.offset(6, 3), b: c.offset(6, 3) });
    sim.push(Command::Build { stuff: Some(wood), thing: wall, a: c.offset(6, 5), b: c.offset(6, 5) });
    sim.push(Command::Build { stuff: Some(wood), thing: door, a: c.offset(6, 4), b: c.offset(6, 4) });
    sim
}

fn load(s: &Snapshot) -> Sim {
    let back = Snapshot::from_bytes(&s.to_bytes()).expect("reads back");
    assert!(back == *s, "the file form round-trips");
    back.restore(&mods(), &|_| true).unwrap_or_else(|e| panic!("loads: {e}"))
}

/// The first section that differs, for a failure worth reading.
fn first_difference(a: &Snapshot, b: &Snapshot) -> String {
    if a.header != b.header {
        return format!("header: {:?} vs {:?}", a.header, b.header);
    }
    let names: std::collections::BTreeSet<&String> = a.sections.keys().chain(b.sections.keys()).collect();
    for n in names {
        let (x, y) = (a.sections.get(n), b.sections.get(n));
        if x != y {
            let at = match (x, y) {
                (Some(x), Some(y)) => x.iter().zip(y).position(|(p, q)| p != q).unwrap_or(x.len().min(y.len())),
                _ => 0,
            };
            return format!("section {n}, from byte {at}");
        }
    }
    "nothing".into()
}

fn assert_same(a: &Snapshot, b: &Snapshot, what: &str) {
    assert!(a == b, "{what}: {}", first_difference(a, b));
}

#[test]
fn save_load_save_gives_the_same_bytes() {
    for seed in [1, 4] {
        let mut sim = colony(seed);
        for _ in 0..TICKS_PER_DAY / 2 + 37 {
            sim.step();
        }
        let first = Snapshot::capture(&sim);
        let again = Snapshot::capture(&load(&first));
        assert_same(&first, &again, &format!("seed {seed}"));
    }
}

#[test]
fn carrying_on_after_a_load_matches_never_saving() {
    for (seed, save_at) in [(1, 3_000), (3, TICKS_PER_DAY / 2 + 11), (5, TICKS_PER_DAY + 4_321), (7, 777)] {
        let mut live = colony(seed);
        for _ in 0..save_at {
            live.step();
        }
        let mut loaded = load(&Snapshot::capture(&live));
        // The loader spawns in id order; this is the worst order hecs could have.
        common::respawn_reversed(&mut loaded);
        // Long enough for raids, deaths and a season change.
        for t in 1..=TICKS_PER_DAY * 8 {
            live.step();
            loaded.step();
            if t % 1000 == 0 {
                let (a, b) = (Snapshot::capture(&live), Snapshot::capture(&loaded));
                assert_same(&a, &b, &format!("seed {seed}, saved at {save_at}, {t} ticks on"));
            }
        }
    }
}

#[test]
fn a_save_from_other_mods_is_refused() {
    let sim = colony(1);
    let s = Snapshot::capture(&sim);
    let err = s.restore(&mods(), &|m| m == "core").err().expect("core alone isn't the same mods");
    assert!(err.contains("mods"), "{err}");
}

const PROBE_SCRIPT: &str = r#"
rim.on("probe:ping", function() rim.emit("probe:pong", {}) end)
rim.on("probe:pong", function() rim.set_data("probe:pongs", (rim.get_data("probe:pongs") or 0) + 1) end)
"#;

const PROBE_DEFS: &str = r##"
[[thing]]
id = "ember"
label = "ember"
color = "#e07b2a"
category = "item"
market_value = 1
stack_limit = 10
emit = [{ field = "core:temperature", amount = 5.0, radius = 2 }]
"##;

fn probe_mods(name: &str) -> PathBuf {
    common::test_mods(
        name,
        &["core", "weather"],
        &[("probe", &[("scripts/probe.luau", PROBE_SCRIPT), ("defs/items.toml", PROBE_DEFS)])],
    )
}

#[test]
fn an_event_raised_by_a_handler_on_the_last_tick_survives() {
    let dir = probe_mods("snap-events");
    let mut live = Sim::new(&dir, 2).expect("loads");
    // ping is handled this tick; the pong it raises waits for the next one.
    live.push(Command::ModEvent { name: "probe:ping".into(), data: None });
    live.step();
    let mut loaded = Snapshot::capture(&live).restore(&dir, &|_| true).expect("restores");
    for _ in 0..3 {
        live.step();
        loaded.step();
    }
    assert!(live.world.data.contains_key("probe:pongs"), "the pong was heard");
    assert_same(&Snapshot::capture(&live), &Snapshot::capture(&loaded), "after a pending event");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_item_that_emits_does_so_before_and_after_a_load() {
    let dir = probe_mods("snap-ember");
    let mut live = Sim::new(&dir, 2).expect("loads");
    let ember = live.world.defs.thing_id("probe:ember").unwrap();
    let c = live.world.colony_center().unwrap();
    live.world.place_item(ember, c.offset(0, 4), 1);
    let loaded = Snapshot::capture(&live).restore(&dir, &|_| true).expect("restores");
    assert_eq!(live.world.fields.emitter_count(), loaded.world.fields.emitter_count());
    assert!(live.world.fields.emitter_count() > 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_command_sees_the_same_map_before_and_after_a_load() {
    let mut live = colony(1);
    for _ in 0..10 {
        live.step();
    }
    let pawn = live.world.colonists().next().unwrap();
    let p = live.world.pawn_pos(pawn).unwrap();
    let wall = live.world.defs.thing_id("wall").unwrap();
    let wood = live.world.defs.thing_id("wood");
    // Wall in a cell between ticks: the regions are out of date until the
    // next tick refreshes them, and the order below is applied before that.
    let open = |q: rim_sim::IVec| live.world.map.passable(q) && live.world.map.fixture_at(q).is_none();
    let target = (3..40)
        .flat_map(|d| [p.offset(d, 0), p.offset(-d, 0), p.offset(0, d), p.offset(0, -d)])
        .find(|&q| open(q) && rim_sim::map::NEIGHBORS8.iter().all(|(dx, dy)| open(q.offset(*dx, *dy))))
        .expect("an open patch near the colonist");
    for (dx, dy) in rim_sim::map::NEIGHBORS8 {
        live.world.spawn_fixture_of(wall, target.offset(dx, dy), false, wood);
    }
    let mut loaded = load(&Snapshot::capture(&live));
    for sim in [&mut live, &mut loaded] {
        sim.push(Command::Order { pawn, cell: target, on: None });
        for _ in 0..30 {
            sim.step();
        }
    }
    assert_same(&Snapshot::capture(&live), &Snapshot::capture(&loaded), "after an order into a walled cell");
}

#[test]
fn script_data_keys_come_back_as_they_were() {
    let mut live = colony(1);
    live.world.data.insert("unprefixed".into(), rim_sim::data::Data::Int(7));
    let loaded = load(&Snapshot::capture(&live));
    assert_eq!(live.world.data, loaded.world.data);
}
