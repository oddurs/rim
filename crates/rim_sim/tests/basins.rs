//! Basins (DESIGN.md §6d): dug space beside the water table floods to the
//! brim, an aquifer fills the level below its breach before its own, water
//! is saved by volume, and a breach is news once.

mod common;

use rim_sim::defs::DefId;
use rim_sim::snapshot::Snapshot;
use rim_sim::water::FULL;
use rim_sim::{IVec, Sim};

fn core() -> Sim {
    Sim::build(&common::mods(), 3, &|m| m == "core", 64).expect("core loads")
}

fn terrain(s: &Sim, id: &str) -> DefId {
    s.world.defs.terrain.iter().position(|t| t.id == format!("core:{id}")).unwrap_or_else(|| panic!("{id}")) as DefId
}

fn set(s: &mut Sim, p: IVec, id: &str) {
    let t = terrain(s, id);
    let cost = s.world.defs.terrain[t as usize].path_cost;
    s.world.map.set_terrain(p, t, cost);
}

/// Dig out `p`: what its rock leaves.
fn dig(s: &mut Sim, p: IVec) {
    if let Some(leaves) = s.world.solid_at(p).and_then(|r| r.leaves_r) {
        let cost = s.world.defs.terrain[leaves as usize].path_cost;
        s.world.map.set_terrain(p, leaves, cost);
    }
}

fn run(s: &mut Sim, ticks: u32, done: impl Fn(&Sim) -> bool) -> bool {
    for _ in 0..ticks {
        s.step();
        if done(s) {
            return true;
        }
    }
    false
}

const WATCH: &str = r#"
rim.on("breach", function(e)
    rim.set_data("breaches", (rim.get_data("breaches") or 0) + 1)
    rim.set_data("source", e.source)
end)
"#;

#[test]
fn digging_beside_the_river_floods_the_dug_space_to_the_brim() {
    let dir = common::test_mods("basins-river", &["core"], &[("watch", &[("scripts/watch.luau", WATCH)])]);
    let mut s = Sim::build(&dir, 3, &|_| true, 64).unwrap();
    // The first tick finds what the map was made with: no breach in that.
    s.step();
    // A pit on the river bank, and a room dug out beside it one level down.
    let pours = |s: &Sim, p: IVec| {
        s.world.map.inb(p) && s.world.defs.terrain[s.world.map.terrain[s.world.map.idx(p)] as usize].pours > 0
    };
    let (bank, away) = (0..64 * 64)
        .map(|i| IVec::new(i % 64, i / 64))
        .filter(|&p| s.world.map.passable(p) && p.x > 8 && p.y > 8 && p.x < 55 && p.y < 55)
        .find_map(|p| {
            let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
            let (dx, dy) = dirs.into_iter().find(|&(dx, dy)| pours(&s, p.offset(dx, dy)))?;
            Some((p, (-dx, -dy)))
        })
        .expect("a river bank");
    let air = terrain(&s, "air");
    s.world.map.set_terrain(bank, air, 0);
    let mut room = Vec::new();
    for k in 0..6 {
        for side in -1..=1 {
            let q = IVec::at(bank.x + away.0 * k + away.1 * side, bank.y + away.1 * k + away.0 * side, -1);
            dig(&mut s, q);
            room.push(q);
        }
    }
    let full = |s: &Sim| room.iter().chain([&bank]).all(|&p| s.world.water_depth(p) == FULL);
    assert!(run(&mut s, 5_000, full), "the room and the pit fill to the brim");
    let breaches = |s: &Sim| match s.world.data.get("watch:breaches") {
        Some(rim_sim::data::Data::Int(n)) => *n,
        _ => 0,
    };
    s.step();
    let seen = breaches(&s);
    let source = format!("{:?}", s.world.data.get("watch:source"));
    assert!(seen >= 1 && source.contains("water"), "a breach, from the water: {seen} {source}");
    for _ in 0..500 {
        s.step();
    }
    assert_eq!(breaches(&s), seen, "and news once, not every tick");
    let _ = std::fs::remove_dir_all(dir);
}

/// A 3 × 3 room at `c` on its level.
fn room(s: &mut Sim, c: IVec) -> Vec<IVec> {
    let cells: Vec<IVec> = (-1..=1).flat_map(|y| (-1..=1).map(move |x| c.offset(x, y))).collect();
    for &p in &cells {
        dig(s, p);
    }
    cells
}

#[test]
fn an_aquifer_breach_at_minus_two_fills_minus_three_first() {
    let mut s = core();
    let c = s.world.colony_center().unwrap();
    let (mid, deep) = (IVec::at(c.x, c.y, -2), IVec::at(c.x, c.y, -3));
    let lower = room(&mut s, deep);
    let upper = room(&mut s, mid);
    // Wet limestone all round the upper room, and a hole in its middle.
    for y in -2..=2 {
        for x in -2..=2 {
            if x == -2 || x == 2 || y == -2 || y == 2 {
                set(&mut s, mid.offset(x, y), "wet_limestone");
            }
        }
    }
    let air = terrain(&s, "air");
    s.world.map.set_terrain(mid, air, 0);
    let depth = |s: &Sim, cells: &[IVec]| cells.iter().map(|&p| s.world.water_depth(p)).max().unwrap_or(0);
    let dry = std::cell::Cell::new(true);
    let filled = run(&mut s, 60_000, |s| {
        let full = lower.iter().all(|&p| s.world.water_depth(p) == FULL);
        if !full && depth(s, &upper) > 0 {
            dry.set(false);
        }
        full
    });
    assert!(filled, "the level below fills");
    assert!(dry.get(), "and the breached level stays dry until it has");
    assert!(run(&mut s, 60_000, |s| depth(s, &upper) > 0), "then it fills too");
}

#[test]
fn water_is_saved_by_volume() {
    let mut s = core();
    let c = s.world.colony_center().unwrap();
    let mid = IVec::at(c.x, c.y, -2);
    let cells = room(&mut s, mid);
    set(&mut s, mid.offset(2, 0), "wet_limestone");
    assert!(run(&mut s, 60_000, |s| s.world.water_depth(mid) >= 2), "some water seeps in");
    let before: Vec<u32> = cells.iter().map(|&p| s.world.water_depth(p)).collect();
    let back = Snapshot::capture(&s).restore(&common::mods(), &|m| m == "core").unwrap();
    let after: Vec<u32> = cells.iter().map(|&p| back.world.water_depth(p)).collect();
    assert_eq!(before, after);
    assert!(Snapshot::capture(&back) == Snapshot::capture(&s), "save, load, save is the same");
}

/// Water reaches the map only every `WATER_EVERY` ticks: a game loaded
/// between two passes has what the last one laid, and knows what was
/// rising, as the game that kept running does.
#[test]
fn a_load_between_water_passes_keeps_what_the_water_did() {
    let mut s = core();
    s.step();
    // A pit on the river bank, and a room dug out beside it one level down,
    // filling from the river.
    let pours = |s: &Sim, p: IVec| {
        s.world.map.inb(p) && s.world.defs.terrain[s.world.map.terrain[s.world.map.idx(p)] as usize].pours > 0
    };
    let (bank, away) = (0..64 * 64)
        .map(|i| IVec::new(i % 64, i / 64))
        .filter(|&p| s.world.map.passable(p) && p.x > 8 && p.y > 8 && p.x < 55 && p.y < 55)
        .find_map(|p| {
            let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
            let (dx, dy) = dirs.into_iter().find(|&(dx, dy)| pours(&s, p.offset(dx, dy)))?;
            Some((p, (-dx, -dy)))
        })
        .expect("a river bank");
    let air = terrain(&s, "air");
    s.world.map.set_terrain(bank, air, 0);
    let mut cells = vec![bank];
    for k in 0..6 {
        for side in -1..=1 {
            let q = IVec::at(bank.x + away.0 * k + away.1 * side, bank.y + away.1 * k + away.0 * side, -1);
            dig(&mut s, q);
            cells.push(q);
        }
    }
    let every = rim_sim::world::WATER_EVERY;
    let q = cells[1];
    // Past swimming depth, and saved between the pass that laid it on the
    // map and the next.
    let swimming = |s: &Sim| s.world.water_depth(q) >= 4 && s.world.tick > every && s.world.tick % every == every / 2;
    assert!(run(&mut s, 5_000, swimming), "water past swimming, between passes");
    let mut back = Snapshot::capture(&s).restore(&common::mods(), &|m| m == "core").unwrap();
    for pass in 0..2 {
        for &p in &cells {
            let (live, loaded) = (&s.world, &back.world);
            assert_eq!(loaded.map.cost(p), live.map.cost(p), "pass {pass}: cost at {p:?}");
            assert_eq!(loaded.map.passable(p), live.map.passable(p), "pass {pass}: footing at {p:?}");
            assert_eq!(loaded.water.rising(&loaded.map, p), live.water.rising(&live.map, p), "pass {pass}: rising");
        }
        for _ in 0..every {
            s.step();
            back.step();
        }
    }
}
