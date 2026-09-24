//! Doors are faction-owned. They open for the colony that built them and
//! stand in everyone else's way, so a wall with a door in it is still a wall.

use rim_sim::hecs::Entity;
use rim_sim::path::Goal;
use rim_sim::world::{Faction, Job, Owner, Pawn, Thing};
use rim_sim::{IVec, Sim};
use std::path::{Path, PathBuf};

fn mods() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

fn sim(seed: u64) -> Sim {
    Sim::new(&mods(), seed).expect("mods load")
}

/// Place a finished fixture, clearing whatever grows there first.
fn put(s: &mut Sim, id: &str, p: IVec) -> Entity {
    if let Some(f) = s.world.map.fixture_at(p) {
        s.world.despawn_thing(f);
    }
    let def = s.world.defs.thing_id(id).unwrap_or_else(|| panic!("core has {id}"));
    s.world.spawn_fixture(def, p, false).unwrap_or_else(|| panic!("could not place {id} at {p:?}"))
}

/// Build it the way the colony would, so it ends up owned.
fn build(s: &mut Sim, id: &str, p: IVec) -> Entity {
    let e = put(s, id, p);
    rim_sim::ai::complete_building(&mut s.world, e);
    e
}

/// A 5x5 hut around `at` with a door in the middle of the bottom wall.
/// Returns the door.
fn hut(s: &mut Sim, at: IVec) -> Entity {
    let o = at.offset(-2, -2);
    let door_at = at.offset(0, 2);
    let mut door = None;
    for y in 0..5 {
        for x in 0..5 {
            let p = o.offset(x, y);
            if !(x == 0 || y == 0 || x == 4 || y == 4) {
                continue;
            }
            if p == door_at {
                door = Some(build(s, "door_wood", p));
            } else {
                build(s, "wall_wood", p);
            }
        }
    }
    s.world.map.ensure_rooms();
    s.world.map.ensure_regions();
    door.expect("a door in the wall")
}

fn run(s: &mut Sim, ticks: u32) {
    for _ in 0..ticks {
        s.step();
    }
}

fn spawn(s: &mut Sim, creature: &str, faction: Faction, p: IVec) -> Entity {
    let def = s.world.defs.creature_id(creature).unwrap_or_else(|| panic!("core has {creature}"));
    s.world.spawn_pawn(def, faction, p, Some("Testrunner".into()))
}

#[test]
fn a_built_door_belongs_to_the_colony() {
    let mut s = sim(21);
    let at = s.world.colony_center().expect("a colony");
    let door = hut(&mut s, at);
    assert_eq!(s.world.ecs.get::<&Owner>(door).map(|o| o.0), Ok(Faction::Player));
    let pos = s.world.thing(door).expect("the door").pos;
    assert_eq!(s.world.map.owner_at(pos), Some(Faction::Player));
}

#[test]
fn the_colony_walks_through_its_own_door() {
    let mut s = sim(21);
    let at = s.world.colony_center().expect("a colony");
    let door = hut(&mut s, at);
    let outside = s.world.thing(door).expect("the door").pos.offset(0, 1);
    assert!(s.world.map.passable_for(outside, Faction::Player), "outside is walkable");
    assert!(
        s.world.map.can_reach_for(outside, Goal::Cell(at), Faction::Player),
        "a colonist can get in through their own door"
    );
}

#[test]
fn a_raider_cannot_walk_through_it() {
    let mut s = sim(21);
    let at = s.world.colony_center().expect("a colony");
    let door = hut(&mut s, at);
    let pos = s.world.thing(door).expect("the door").pos;
    assert!(!s.world.map.passable_for(pos, Faction::Hostile), "the door is shut to raiders");
    assert!(
        !s.world.map.can_reach_for(pos.offset(0, 1), Goal::Cell(at), Faction::Hostile),
        "and there is no way round it"
    );
    // The pathfinder has to agree with the region layer, or pawns stall.
    let w = &mut s.world;
    let from = pos.offset(0, 1);
    assert!(w.pf.find(&w.map, from, Goal::Cell(at), 20_000, Faction::Hostile).is_none(), "no path for a raider");
    assert!(w.pf.find(&w.map, from, Goal::Cell(at), 20_000, Faction::Player).is_some(), "but one for the owner");
}

#[test]
fn an_unowned_door_opens_for_anyone() {
    let mut s = sim(21);
    let at = s.world.colony_center().expect("a colony");
    let door = hut(&mut s, at);
    let pos = s.world.thing(door).expect("the door").pos;
    // A ruin: the same door with nobody's name on it.
    s.world.map.set_owner(pos, None);
    let _ = s.world.ecs.remove_one::<Owner>(door);
    s.world.map.ensure_regions();
    assert!(s.world.map.passable_for(pos, Faction::Hostile), "nobody owns it, so it opens");
}

#[test]
fn a_walled_out_raider_goes_for_the_door() {
    let mut s = sim(21);
    let at = s.world.colony_center().expect("a colony");
    // Move every colonist inside before sealing the hut.
    for e in s.world.colonists().collect::<Vec<_>>() {
        if let Ok(mut p) = s.world.ecs.get::<&mut Pawn>(e) {
            p.pos = at;
            p.next = None;
            p.path.clear();
            p.path_goal = None;
        }
    }
    let door = hut(&mut s, at);
    let outside = s.world.thing(door).expect("the door").pos.offset(0, 3);
    let raider = spawn(&mut s, "human", Faction::Hostile, outside);

    run(&mut s, 120);
    let job = s.world.ecs.get::<&Pawn>(raider).expect("alive").job.clone();
    assert!(matches!(job, Job::Breach { door: d } if d == door), "a shut-out raider breaks the door: {job:?}");
}

#[test]
fn breaking_the_door_opens_the_way_in() {
    let mut s = sim(21);
    let at = s.world.colony_center().expect("a colony");
    for e in s.world.colonists().collect::<Vec<_>>() {
        if let Ok(mut p) = s.world.ecs.get::<&mut Pawn>(e) {
            p.pos = at;
            p.next = None;
            p.path.clear();
            p.path_goal = None;
        }
    }
    let door = hut(&mut s, at);
    let outside = s.world.thing(door).expect("the door").pos.offset(0, 3);
    let raider = spawn(&mut s, "human", Faction::Hostile, outside);
    // The colony comes out to meet them (see below), and a lone raider
    // loses that fight long before the door gives. This test is about the
    // door, so give them the staying power to finish the job.
    if let Ok(mut p) = s.world.ecs.get::<&mut Pawn>(raider) {
        p.hp = 100_000;
    }

    let mut broken = false;
    for _ in 0..6000 {
        s.step();
        if s.world.thing(door).is_none() {
            broken = true;
            break;
        }
    }
    assert!(broken, "the raider should get through eventually");
    let texts: Vec<_> = s.world.messages.iter().map(|m| m.text.clone()).collect();
    assert!(texts.iter().any(|t| t.contains("broken down")), "the player is told: {texts:?}");
    s.world.map.ensure_regions();
    assert!(s.world.map.can_reach_for(outside, Goal::Cell(at), Faction::Hostile), "the way in is open now");
}

#[test]
fn a_door_in_open_ground_blocks_nothing() {
    let mut s = sim(21);
    let at = s.world.colony_center().expect("a colony");
    let spot = (1..12)
        .flat_map(|r| [at.offset(r, 0), at.offset(-r, 0), at.offset(0, r), at.offset(0, -r)])
        .find(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none())
        .expect("open ground");
    build(&mut s, "door_wood", spot);
    s.world.map.ensure_regions();
    // Shut to a raider, but they can simply walk around it.
    assert!(!s.world.map.passable_for(spot, Faction::Hostile));
    assert!(
        s.world.map.can_reach_for(spot.offset(0, -2), Goal::Touch(at), Faction::Hostile),
        "one door on open ground is not a wall"
    );
}

#[test]
fn a_thing_query_still_sees_the_door_until_it_breaks() {
    let mut s = sim(21);
    let at = s.world.colony_center().expect("a colony");
    let door = hut(&mut s, at);
    let before = s.world.ecs.query::<&Thing>().iter().count();
    s.world.despawn_thing(door);
    s.world.map.ensure_regions();
    assert_eq!(s.world.ecs.query::<&Thing>().iter().count(), before - 1);
    let pos = at.offset(0, 2);
    assert_eq!(s.world.map.owner_at(pos), None, "ownership goes with the door");
    assert!(s.world.map.passable_for(pos, Faction::Hostile), "the gap is open to all");
}

/// The other half of ownership: a door is only a wall to the people who do
/// not own it. The colony walks out through its own door to fight.
#[test]
fn the_colony_comes_out_through_its_own_door() {
    let mut s = sim(21);
    let at = s.world.colony_center().expect("a colony");
    for e in s.world.colonists().collect::<Vec<_>>() {
        if let Ok(mut p) = s.world.ecs.get::<&mut Pawn>(e) {
            p.pos = at;
            p.next = None;
            p.path.clear();
            p.path_goal = None;
        }
    }
    let door = hut(&mut s, at);
    let outside = s.world.thing(door).expect("the door").pos.offset(0, 3);
    let raider = spawn(&mut s, "human", Faction::Hostile, outside);

    let mut fought = false;
    for _ in 0..2000 {
        s.step();
        let hurt = s.world.ecs.get::<&Pawn>(raider).map_or(true, |p| p.hp < 100 || p.dead);
        if hurt {
            fought = true;
            break;
        }
    }
    assert!(fought, "the defenders should be able to get at a raider on their doorstep");
    assert!(s.world.thing(door).is_some(), "and without knocking their own door down");
}
