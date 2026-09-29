//! The pathfinder's landmark bound (`path::Landmarks`): searches round a
//! lake expand far fewer cells and find paths just as short, and the
//! landmarks follow a change to the land on the same tick in a saved game
//! as in one that never saved.

use rim_sim::path::{Goal, Pathfinder, LAND_DELAY};
use rim_sim::snapshot::Snapshot;
use rim_sim::world::Faction;
use rim_sim::{IVec, Sim};
use std::path::{Path, PathBuf};

fn mods() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

fn deep_water(sim: &Sim) -> Vec<IVec> {
    let deep = sim.world.defs.lookup("terrain", "deep_water").unwrap();
    let m = &sim.world.map;
    (0..m.plane()).map(|i| m.pos(i)).filter(|&p| m.terrain[m.idx(p)] == deep).collect()
}

/// The first seed whose map has a lake worth going round.
fn lakeside() -> Sim {
    (1..40)
        .map(|seed| Sim::new(&mods(), seed).expect("mods load"))
        .find(|s| deep_water(s).len() > 400)
        .expect("a seed with a lake")
}

/// What a path costs, step by step, as the search charges it.
fn cost(sim: &Sim, start: IVec, path: &[IVec]) -> u32 {
    let m = &sim.world.map;
    let mut at = start;
    let mut total = 0;
    for &p in path.iter().rev() {
        assert_eq!(p.z, at.z, "a surface path");
        let diag = p.x != at.x && p.y != at.y;
        total += ((if diag { 14 } else { 10 }) * m.cost(p) / 100).max(1);
        at = p;
    }
    total
}

/// Pairs of open surface cells spread over the map, far apart.
fn pairs(sim: &Sim, n: usize) -> Vec<(IVec, IVec)> {
    let m = &sim.world.map;
    let open: Vec<IVec> = (0..m.plane()).map(|i| m.pos(i)).filter(|&p| m.passable(p)).collect();
    (0..n)
        .map(|k| (open[k * 7919 % open.len()], open[(k * 104_729 + open.len() / 2) % open.len()]))
        .filter(|(a, b)| a.chebyshev(*b) > 20)
        .collect()
}

#[test]
fn round_a_lake_searches_expand_less_and_find_paths_as_short() {
    let mut sim = lakeside();
    sim.world.map.ensure_regions();
    assert!(sim.world.pf.landmarks().is_some(), "a new game has its landmarks");
    let bytes = sim.world.pf.landmarks().unwrap().bytes();
    println!("landmarks: {bytes} bytes for {} cells", sim.world.map.cells());
    let mut plain = Pathfinder::default();
    let (mut crossed, mut with, mut without) = (0, 0, 0);
    for (a, b) in pairs(&sim, 200) {
        if !sim.world.map.can_reach(a, Goal::Cell(b)) {
            continue;
        }
        let before = (sim.world.pf.expanded, plain.expanded);
        let fast = sim.world.pf.find(&sim.world.map, a, Goal::Cell(b), 1_000_000, Faction::Player).expect("a way");
        let slow = plain.find(&sim.world.map, a, Goal::Cell(b), 1_000_000, Faction::Player).expect("a way");
        assert_eq!(cost(&sim, a, &fast), cost(&sim, a, &slow), "{a:?} to {b:?}: as short a path");
        with += sim.world.pf.expanded - before.0;
        without += plain.expanded - before.1;
        crossed += 1;
    }
    println!("{crossed} searches: {with} cells expanded with the landmarks, {without} without");
    assert!(crossed > 50, "enough searches: {crossed}");
    assert!(with * 2 <= without, "at least halved: {with} against {without}");
}

/// Make a deep-water cell grass: the land changes.
fn drain(sim: &mut Sim, p: IVec) {
    let grass = sim.world.defs.lookup("terrain", "grass").unwrap();
    let cost = sim.world.defs.terrain[grass as usize].path_cost;
    sim.world.map.set_terrain(p, grass, cost);
}

#[test]
fn the_landmarks_wait_for_a_change_to_the_land_and_start_again_at_a_second() {
    let mut sim = lakeside();
    let water = deep_water(&sim);
    sim.step();
    assert!(sim.world.pf.landmarks().is_some() && sim.world.land_changed.is_none());

    drain(&mut sim, water[0]);
    sim.step();
    let first = sim.world.land_changed.expect("the change is seen at the end of its tick");
    assert_eq!(first, sim.world.tick);
    assert!(sim.world.pf.landmarks().is_none(), "the octile distance alone meanwhile");
    for _ in 0..50 {
        sim.step();
    }
    // A second change inside the wait: the first build is dropped, and the
    // wait runs from the second.
    drain(&mut sim, water[1]);
    sim.step();
    let second = sim.world.land_changed.expect("waiting");
    assert_eq!(second, first + 51);
    while sim.world.tick < second + LAND_DELAY - 1 {
        sim.step();
        assert!(sim.world.pf.landmarks().is_none(), "still waiting at {}", sim.world.tick);
    }
    sim.step();
    assert_eq!(sim.world.tick, second + LAND_DELAY);
    assert!(sim.world.pf.landmarks().is_some() && sim.world.land_changed.is_none(), "switched on the tick");
}

#[test]
fn a_game_saved_while_the_landmarks_wait_goes_on_as_one_that_never_saved() {
    let mut live = lakeside();
    let water = deep_water(&live);
    for _ in 0..10 {
        live.step();
    }
    drain(&mut live, water[0]);
    live.step();
    let changed = live.world.land_changed.expect("waiting");
    for _ in 0..60 {
        live.step();
    }
    let saved = Snapshot::capture(&live);
    let mut loaded = Snapshot::from_bytes(&saved.to_bytes()).unwrap().restore(&mods(), &|_| true).expect("loads");
    assert_eq!(loaded.world.land_changed, Some(changed), "the wait is saved");
    assert!(loaded.world.pf.landmarks().is_none(), "and kept");
    for t in 1..=200 {
        live.step();
        loaded.step();
        assert_eq!(
            live.world.pf.landmarks().is_some(),
            loaded.world.pf.landmarks().is_some(),
            "{t} ticks on: switched on the same tick"
        );
        if t % 20 == 0 {
            assert!(Snapshot::capture(&live) == Snapshot::capture(&loaded), "{t} ticks on: the same game");
        }
    }
    assert!(live.world.pf.landmarks().is_some());
}

/// A floor cheaper than the ground counts down to open ground's 100 and
/// no further, as the octile distance counts every step: on grass it
/// changes nothing, so laying one rebuilds nothing; on sand it does.
#[test]
fn a_floor_counts_no_cheaper_than_open_ground() {
    let mut sim = lakeside();
    let defs = sim.world.defs.clone();
    let floor = defs.thing_id("floor").unwrap();
    let cost = defs.thing(floor).path_cost;
    assert!(cost < 100, "core's floor is cheaper than open ground: {cost}");
    let m = &mut sim.world.map;
    let on = |m: &rim_sim::map::Map, id: &str| {
        let t = defs.lookup("terrain", id).unwrap();
        (0..m.plane()).map(|i| m.pos(i)).find(|&p| m.terrain[m.idx(p)] == t).expect(id)
    };
    for (ground, before, after, rebuilds) in [("grass", 100, 100, false), ("sand", 130, 100, true)] {
        let p = on(m, ground);
        let i = m.idx(p);
        assert_eq!(m.land(i), Some(before), "{ground}");
        let rev = m.land_rev();
        m.set_floor(p, Some(rim_sim::hecs::Entity::DANGLING), cost);
        assert_eq!(m.land(i), Some(after), "{ground} with a floor");
        assert_eq!(m.land_rev() != rev, rebuilds, "{ground}: whether the landmarks rebuild");
    }
}
