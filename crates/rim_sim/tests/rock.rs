//! Rock is terrain until someone works it (DESIGN.md §6d): a solid cell is
//! no entity, marking it stands its thing up, and mining the thing leaves
//! the terrain's floor.

mod common;

use rim_sim::path::Goal;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{Thing, Work};
use rim_sim::{Command, IVec, Sim};

/// Core alone: rock is mined bare-handed.
fn core(seed: u64) -> Sim {
    Sim::build(&common::mods(), seed, &|m| m == "core", 200).expect("core loads")
}

fn granite_things(s: &Sim) -> usize {
    let granite = s.world.defs.thing_id("granite").unwrap();
    s.world.ecs.query::<&Thing>().iter().filter(|t| t.def == granite).count()
}

fn mark(s: &mut Sim, rock: IVec) {
    let mine = s.world.defs.lookup("designation", "core:mine").unwrap();
    s.push(Command::Designate { designation: mine, a: rock, b: rock });
}

fn run_until(s: &mut Sim, ticks: u32, done: impl Fn(&Sim) -> bool) -> bool {
    for _ in 0..ticks {
        s.step();
        if done(s) {
            return true;
        }
    }
    false
}

#[test]
fn map_generation_makes_no_rock_entities() {
    let s = core(1);
    let solid = (0..s.world.map.w * s.world.map.h).filter(|&i| s.world.solid_at(s.world.map.pos(i as usize)).is_some());
    assert!(solid.count() > 500, "the seed has hills");
    assert_eq!(granite_things(&s), 0, "rock is terrain, not things");
}

#[test]
fn mining_rock_leaves_its_floor_and_opens_the_way() {
    let mut s = core(1);
    let c = s.world.colony_center().unwrap();
    let rock = common::nearest_rock(&s.world, c).expect("rock in reach");
    assert!(!s.world.map.passable(rock));
    mark(&mut s, rock);
    s.step();
    assert!(s.world.map.fixture_at(rock).is_some(), "marking stands it up");
    assert_eq!(granite_things(&s), 1, "only the marked cell");

    assert!(run_until(&mut s, 20_000, |s| s.world.solid_at(rock).is_none()), "mined");
    let floor = s.world.defs.lookup("terrain", "core:rock_floor").unwrap();
    assert_eq!(s.world.map.terrain[s.world.map.idx(rock)], floor, "leaves rough stone");
    assert!(s.world.map.fixture_at(rock).is_none() && s.world.map.passable(rock));
    s.world.map.ensure_regions();
    assert!(s.world.map.can_reach(c, Goal::Cell(rock)), "the way is open");
    let stone = s.world.defs.thing_id("stone").unwrap();
    let near = s.world.ecs.query::<&Thing>().iter().any(|t| t.def == stone && t.pos.chebyshev(rock) <= 2);
    assert!(near, "it yields stone where it stood");
}

#[test]
fn unmarking_untouched_rock_leaves_it_terrain() {
    let mut s = core(1);
    let rock = common::nearest_rock(&s.world, s.world.colony_center().unwrap()).unwrap();
    // Both land on the same tick boundary, before anyone takes the work.
    mark(&mut s, rock);
    s.push(Command::Cancel { a: rock, b: rock });
    s.step();
    assert!(s.world.map.fixture_at(rock).is_none(), "back to terrain");
    assert!(s.world.solid_at(rock).is_some(), "still rock");
    assert_eq!(granite_things(&s), 0);
}

#[test]
fn mining_progress_survives_a_save_and_load() {
    let mut s = core(1);
    let rock = common::nearest_rock(&s.world, s.world.colony_center().unwrap()).unwrap();
    mark(&mut s, rock);
    let begun = |s: &Sim| {
        s.world.map.fixture_at(rock).and_then(|e| s.world.ecs.get::<&Work>(e).ok().map(|w| w.done)).unwrap_or(0) > 0
    };
    assert!(run_until(&mut s, 20_000, begun), "someone starts on it");
    let done = |s: &Sim| s.world.ecs.get::<&Work>(s.world.map.fixture_at(rock).unwrap()).unwrap().done;
    let before = done(&s);

    let mut loaded = Snapshot::capture(&s).restore(&common::mods(), &|m| m == "core").expect("loads");
    assert_eq!(done(&loaded), before, "the work done on it is kept");
    assert!(run_until(&mut loaded, 20_000, |s| s.world.solid_at(rock).is_none()), "and finished after the load");
}
