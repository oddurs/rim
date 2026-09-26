//! A building planned over grass, a tree or rock: the thing is marked to be
//! cleared, and the blueprint goes up once it's gone. Nothing is silently
//! left out of a wall.

mod common;

use rim_sim::hecs::Entity;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{Blueprint, Designated, Planned, Thing};
use rim_sim::{Command, IVec, Sim};

fn alone(sim: &mut Sim) -> Entity {
    sim.step();
    let founder = sim.world.colonists().next().expect("a founder");
    for e in sim.world.pawns.clone() {
        if e != founder {
            let _ = sim.world.ecs.despawn(e);
        }
    }
    sim.world.pawns.retain(|&e| e == founder);
    founder
}

/// The nearest reachable thing of a def with open, fixture-free cells on
/// either side of it (for a row of three).
fn in_a_row(s: &Sim, id: &str) -> Option<(Entity, IVec)> {
    let d = s.world.defs.thing_id(id).unwrap();
    let c = s.world.colony_center().unwrap();
    let open = |p: IVec| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none();
    s.world
        .ecs
        .query::<(Entity, &Thing)>()
        .iter()
        .filter(|(_, t)| t.def == d && open(t.pos.offset(-1, 0)) && open(t.pos.offset(1, 0)))
        .filter(|(_, t)| s.world.map.can_reach(c, rim_sim::path::Goal::Touch(t.pos)))
        .map(|(e, t)| (e, t.pos))
        .min_by_key(|(e, p)| (p.octile(c), e.id()))
}

fn wall_at(s: &Sim, p: IVec) -> Option<Entity> {
    let wall = s.world.defs.thing_id("core:wall").unwrap();
    s.world.map.fixture_at(p).filter(|&f| s.world.thing(f).is_some_and(|t| t.def == wall))
}

fn built(s: &Sim, p: IVec) -> bool {
    wall_at(s, p).is_some_and(|f| s.world.ecs.get::<&Blueprint>(f).is_err())
}

fn build_wall(s: &mut Sim, a: IVec, b: IVec) {
    let (wall, wood) = (s.world.defs.thing_id("core:wall").unwrap(), s.world.defs.thing_id("core:wood").unwrap());
    s.push(Command::Build { thing: wall, stuff: Some(wood), a, b });
    s.step();
}

#[test]
fn a_wall_dragged_across_tall_grass_has_no_gap() {
    let mut s = Sim::new(&common::mods(), 3).unwrap();
    let founder = alone(&mut s);
    let (grass, at) = in_a_row(&s, "primitive:tall_grass").expect("tall grass with room either side");
    let home = s.world.pawn_pos(founder).unwrap();
    s.world.place_item(s.world.defs.thing_id("core:wood").unwrap(), home, 30);
    build_wall(&mut s, at.offset(-1, 0), at.offset(1, 0));
    assert!(wall_at(&s, at.offset(-1, 0)).is_some() && wall_at(&s, at.offset(1, 0)).is_some());
    assert!(s.world.ecs.get::<&Planned>(grass).is_ok(), "the grass is planned over");
    let gather = s.world.defs.lookup("designation", "core:gather").unwrap();
    assert_eq!(s.world.ecs.get::<&Designated>(grass).map(|d| d.0).ok(), Some(gather), "and marked to be cleared");
    // Nothing is set down where the wall will go: a neighbour's yield
    // would be buried under it.
    s.world.place_item(s.world.defs.thing_id("primitive:fibre").unwrap(), at, 3);
    assert!(s.world.map.item_at(at).is_none(), "set down beside the plan, not on it");
    for _ in 0..12_000 {
        s.step();
        if (-1..=1).all(|dx| built(&s, at.offset(dx, 0))) {
            break;
        }
    }
    assert!((-1..=1).all(|dx| built(&s, at.offset(dx, 0))), "three walls, no gap");
    assert!(s.world.thing(grass).is_none());
    assert!(s.world.map.item_at(at).is_none(), "the fibre it gave isn't buried under the wall");
}

/// Grass in every cell of the row: what the first gives lies clear of the
/// cells still waiting for their wall.
#[test]
fn a_row_of_grass_buries_nothing() {
    let mut s = Sim::new(&common::mods(), 3).unwrap();
    let founder = alone(&mut s);
    let home = s.world.pawn_pos(founder).unwrap();
    let grass = s.world.defs.thing_id("primitive:tall_grass").unwrap();
    let open = |s: &Sim, p: IVec| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none();
    let start = (3..30)
        .flat_map(|d| [home.offset(d, 0), home.offset(-d, 0), home.offset(0, d), home.offset(0, -d)])
        .find(|&p| (-1..5).all(|dx| (-1..=1).all(|dy| open(&s, p.offset(dx, dy)) && p.offset(dx, dy) != home)))
        .expect("open ground");
    for dx in 0..4 {
        s.world.spawn_fixture_of(grass, start.offset(dx, 0), false, None).unwrap();
    }
    s.world.place_item(s.world.defs.thing_id("core:wood").unwrap(), home, 40);
    build_wall(&mut s, start, start.offset(3, 0));
    for _ in 0..20_000 {
        s.step();
        if (0..4).all(|dx| built(&s, start.offset(dx, 0))) {
            break;
        }
    }
    assert!((0..4).all(|dx| built(&s, start.offset(dx, 0))), "four walls");
    assert!((0..4).all(|dx| s.world.map.item_at(start.offset(dx, 0)).is_none()), "no fibre under any of them");
}

/// Marking a planned oak for gathering doesn't take the chop off it: the
/// oak would stand, and the wall wait, forever.
#[test]
fn gathering_over_a_planned_oak_keeps_it_marked_to_fell() {
    let mut s = Sim::new(&common::mods(), 3).unwrap();
    let founder = alone(&mut s);
    let (oak, at) = in_a_row(&s, "core:tree_oak").expect("an oak");
    let home = s.world.pawn_pos(founder).unwrap();
    s.world.place_item(s.world.defs.thing_id("primitive:hand_axe").unwrap(), home, 1);
    s.world.place_item(s.world.defs.thing_id("core:wood").unwrap(), home, 10);
    build_wall(&mut s, at, at);
    let (chop, gather) = (
        s.world.defs.lookup("designation", "core:chop").unwrap(),
        s.world.defs.lookup("designation", "core:gather").unwrap(),
    );
    s.push(Command::Designate { designation: gather, a: at, b: at });
    s.step();
    assert_eq!(s.world.ecs.get::<&Designated>(oak).map(|d| d.0).ok(), Some(chop));
    for _ in 0..20_000 {
        s.step();
        if built(&s, at) {
            break;
        }
    }
    assert!(built(&s, at), "felled, and the wall stands where it was");
}

/// With no harvest that clears it (a berry bush regrows), a thing in the
/// way is cleared at once.
#[test]
fn a_bush_with_no_clearing_harvest_is_cleared_at_once() {
    let mut s = Sim::with_mods(&common::mods(), 3, &|m| m == "core").unwrap();
    alone(&mut s);
    let (bush, at) = in_a_row(&s, "core:berry_bush").expect("a berry bush");
    build_wall(&mut s, at, at);
    assert!(s.world.thing(bush).is_none(), "cleared");
    assert!(wall_at(&s, at).is_some_and(|f| s.world.ecs.get::<&Blueprint>(f).is_ok()), "and planned");
}

/// Cancelling the plan leaves the tree: nothing but the plan wanted it gone.
#[test]
fn cancelling_a_plan_over_an_oak_leaves_the_oak() {
    let mut s = Sim::with_mods(&common::mods(), 3, &|m| m == "core").unwrap();
    alone(&mut s);
    let (oak, at) = in_a_row(&s, "core:tree_oak").expect("an oak");
    build_wall(&mut s, at, at);
    assert!(s.world.ecs.get::<&Planned>(oak).is_ok() && s.world.ecs.get::<&Designated>(oak).is_ok());
    s.push(Command::Cancel { a: at, b: at });
    s.step();
    assert!(s.world.ecs.get::<&Planned>(oak).is_err() && s.world.ecs.get::<&Designated>(oak).is_err());
    for _ in 0..3_000 {
        s.step();
    }
    assert!(s.world.thing(oak).is_some(), "still standing");
    assert!(wall_at(&s, at).is_none());
}

#[test]
fn a_plan_survives_a_save() {
    let mods = common::mods();
    let mut s = Sim::with_mods(&mods, 3, &|m| m == "core").unwrap();
    alone(&mut s);
    let (oak, at) = in_a_row(&s, "core:tree_oak").expect("an oak");
    build_wall(&mut s, at, at);
    let snap = Snapshot::capture(&s);
    let back = snap.restore(&mods, &|m| m == "core").unwrap_or_else(|e| panic!("restores: {e}"));
    let wall = back.world.defs.thing_id("core:wall").unwrap();
    assert_eq!(back.world.ecs.get::<&Planned>(oak).map(|p| p.thing).ok(), Some(wall));
    assert_eq!(Snapshot::capture(&back).hash(), snap.hash());
}

/// A floor lies under plants: planning one over grass leaves the grass.
#[test]
fn a_floor_over_grass_leaves_the_grass() {
    let mut s = Sim::new(&common::mods(), 3).unwrap();
    alone(&mut s);
    let (grass, at) = in_a_row(&s, "primitive:tall_grass").expect("tall grass");
    let floor = s.world.defs.thing_id("core:floor").unwrap();
    let wood = s.world.defs.thing_id("core:wood").unwrap();
    s.push(Command::Build { thing: floor, stuff: Some(wood), a: at, b: at });
    s.step();
    assert!(s.world.map.floor_at(at).is_some(), "the floor is planned");
    assert!(s.world.ecs.get::<&Planned>(grass).is_err() && s.world.thing(grass).is_some());
}
