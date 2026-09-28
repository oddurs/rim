//! Trenches and bridges (DESIGN.md §6d): air is no footing, a bridge is,
//! raiders shut out by a trench bridge it a cell at a time, a drawbridge
//! opens for its owner only, and what stands on a bridge that goes falls.

mod common;

use rim_sim::command::{build_preview, Blocker, Place};
use rim_sim::path::Goal;
use rim_sim::world::{Faction, Owner, Pawn, Thing};
use rim_sim::{Command, IVec, Sim};

fn core() -> Sim {
    Sim::build(&common::mods(), 3, &|m| m == "core", 64).expect("core loads")
}

fn thing(s: &Sim, id: &str) -> rim_sim::defs::DefId {
    s.world.defs.thing_id(id).unwrap_or_else(|| panic!("thing {id}"))
}

/// Make `p` a pit: air, with the cell below dug out to land in.
fn pit(s: &mut Sim, p: IVec) {
    let w = &mut s.world;
    for e in [w.map.fixture_at(p), w.map.item_at(p), w.map.floor_at(p)].into_iter().flatten() {
        w.despawn_thing(e);
    }
    let air = w.defs.terrain.iter().position(|t| t.air).expect("core has air") as rim_sim::defs::DefId;
    w.map.set_terrain(p, air, 0);
    let below = IVec::at(p.x, p.y, p.z - 1);
    if let Some(leaves) = w.solid_at(below).and_then(|r| r.leaves_r) {
        let cost = w.defs.terrain[leaves as usize].path_cost;
        w.map.set_terrain(below, leaves, cost);
    }
}

/// A ring of pits `r..=r + width - 1` cells out from `c`.
fn trench(s: &mut Sim, c: IVec, r: i32, width: i32) {
    for y in -(r + width)..=(r + width) {
        for x in -(r + width)..=(r + width) {
            let d = x.abs().max(y.abs());
            if d >= r && d < r + width {
                pit(s, c.offset(x, y));
            }
        }
    }
}

/// Everyone gone but the founder, drafted so they hold still.
fn founder_alone(s: &mut Sim) -> rim_sim::hecs::Entity {
    let founder = s.world.colonists().next().unwrap();
    for e in s.world.pawns.clone() {
        if e != founder {
            let _ = s.world.ecs.despawn(e);
        }
    }
    s.world.pawns.retain(|&e| e == founder);
    s.push(Command::Draft { pawn: founder, on: true });
    s.step();
    founder
}

#[test]
fn air_is_no_footing_and_a_bridge_is() {
    let mut s = core();
    let c = s.world.colony_center().unwrap();
    let p = c.offset(4, 0);
    pit(&mut s, p);
    assert!(!s.world.map.passable(p), "a pit is no footing");
    let bridge = thing(&s, "bridge");
    let wood = s.world.defs.materials("structural")[0];
    s.world.spawn_fixture_of(bridge, p, false, Some(wood)).expect("a bridge over the pit");
    assert!(s.world.map.passable(p), "a bridge is");
    // Only over a pit.
    let ground = c.offset(-4, 0);
    let on_ground = build_preview(&s.world, bridge, Some(wood), ground, ground, 0);
    assert_eq!(on_ground, vec![(ground, Place::Blocked(Blocker::NotOverAir))]);
    let q = c.offset(4, 1);
    pit(&mut s, q);
    assert_eq!(build_preview(&s.world, bridge, Some(wood), q, q, 0), vec![(q, Place::Open)]);
}

#[test]
fn what_stands_on_a_bridge_that_goes_falls_to_the_level_below() {
    let mut s = core();
    let founder = founder_alone(&mut s);
    let c = s.world.colony_center().unwrap();
    let p = c.offset(3, 3);
    pit(&mut s, p);
    let wood = s.world.defs.materials("structural")[0];
    let bridge = s.world.spawn_fixture_of(thing(&s, "bridge"), p, false, Some(wood)).unwrap();
    let stone = thing(&s, "stone");
    s.world.place_item(stone, p, 3);
    s.world.ecs.get::<&mut Pawn>(founder).unwrap().pos = p;
    let hp = s.world.ecs.get::<&Pawn>(founder).unwrap().hp;
    s.world.despawn_thing(bridge);
    let below = IVec::at(p.x, p.y, -1);
    let fp = s.world.ecs.get::<&Pawn>(founder).unwrap().pos;
    assert_eq!(fp, below, "the founder fell a level");
    assert!(s.world.ecs.get::<&Pawn>(founder).unwrap().hp < hp, "and was hurt");
    let landed = s.world.ecs.query::<&Thing>().iter().any(|t| t.def == stone && t.pos.z == -1);
    assert!(landed, "the stone fell with them");
    // Dropped over the pit later, it lands below too.
    s.world.place_item(stone, p, 2);
    assert!(s.world.map.item_at(p).is_none(), "nothing lies on air");
}

#[test]
fn a_drawbridge_opens_for_its_owner_and_is_the_pit_to_raiders() {
    let mut s = core();
    let c = s.world.colony_center().unwrap();
    trench(&mut s, c, 6, 2);
    // A drawbridge across both cells of the trench's width, east.
    let draw = thing(&s, "drawbridge");
    let wood = s.world.defs.materials("structural")[0];
    for x in [6, 7] {
        let p = c.offset(x, 0);
        let e = s.world.spawn_fixture_of(draw, p, false, Some(wood)).expect("a drawbridge");
        let _ = s.world.ecs.insert_one(e, Owner(Faction::Player));
        s.world.map.set_owner(p, Some(Faction::Player));
    }
    s.world.map.ensure_regions();
    let out = c.offset(12, 0);
    assert!(s.world.map.can_reach_for(c, Goal::Cell(out), Faction::Player), "colonists get out");
    assert!(!s.world.map.can_reach_for(out, Goal::Cell(c), Faction::Hostile), "raiders don't get in");
}

#[test]
fn a_raid_against_a_colony_ringed_by_a_trench_bridges_it_and_arrives() {
    let mut s = core();
    let founder = founder_alone(&mut s);
    let c = s.world.ecs.get::<&Pawn>(founder).unwrap().pos;
    trench(&mut s, c, 5, 2);
    s.world.map.ensure_regions();
    let far = c.offset(14, 0);
    assert!(!s.world.map.can_reach_for(far, Goal::Touch(c), Faction::Hostile), "the trench shuts them out");
    let human = s.world.defs.start.as_ref().unwrap().creature_r;
    let spot = (0..8).map(|k| far.offset(0, k)).find(|&p| s.world.map.passable(p)).expect("ground for the raider");
    let raider = s.world.spawn_pawn(human, Faction::Hostile, spot, None);
    let bridge = thing(&s, "bridge");
    let mut arrived = false;
    for _ in 0..20_000 {
        s.step();
        let Some(rp) = s.world.pawn_pos(raider) else { break };
        let Some(fp) = s.world.pawn_pos(founder) else { break };
        if rp.chebyshev(fp) <= 1 {
            arrived = true;
            break;
        }
    }
    let bridged = s.world.ecs.query::<&Thing>().iter().filter(|t| t.def == bridge).count();
    assert!(bridged >= 2, "the raider bridged the trench's width ({bridged} cells)");
    assert!(arrived, "and reached the founder");
}

#[test]
fn colonists_build_a_bridge_over_a_pit_and_walk_it() {
    let mut s = core();
    let c = s.world.colony_center().unwrap();
    // A pit beside the colony, and the material on hand.
    let p = (2..10).map(|r| c.offset(r, 0)).find(|&q| s.world.map.passable(q)).unwrap();
    pit(&mut s, p);
    let wood = s.world.defs.materials("structural")[0];
    s.world.place_item(wood, c, 20);
    let bridge = thing(&s, "bridge");
    s.push(Command::Build { thing: bridge, stuff: Some(wood), a: p, b: p, facing: 0 });
    let built =
        |s: &Sim| s.world.map.floor_at(p).is_some_and(|f| s.world.ecs.get::<&rim_sim::world::Blueprint>(f).is_err());
    let mut done = false;
    for _ in 0..40_000 {
        s.step();
        if built(&s) {
            done = true;
            break;
        }
    }
    assert!(done, "the bridge was built");
    assert!(s.world.map.passable(p), "and is walked on");
}
