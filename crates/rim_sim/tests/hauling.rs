//! Hauling: colonists carry loose items to the stockpiles that take them.

mod common;

use rim_sim::hecs::Entity;
use rim_sim::world::{Blueprint, Job, Pawn, Thing};
use rim_sim::{Command, IVec, Sim};

/// A colony whose only work is hauling unless a test adds more: every
/// other work type is set to 0, and there's an open patch for a stockpile.
fn colony() -> (Sim, Entity, IVec) {
    let mut sim = Sim::new(&common::mods(), 5).unwrap();
    let pawn = sim.world.colonists().next().unwrap();
    let defs = sim.world.defs.clone();
    for (w, d) in defs.work_types.iter().enumerate() {
        let level = if d.id == "core:haul" { 1 } else { 0 };
        sim.push(Command::SetPriority { pawn, work: w as rim_sim::defs::DefId, level });
    }
    let c = sim.world.pawn_pos(pawn).unwrap();
    // Room for items: open, and nothing standing there (tall grass is
    // passable, but no stack goes under it).
    let open = |s: &Sim, o: IVec| {
        (0..3).all(|x| {
            (0..3).all(|y| {
                let p = o.offset(x, y);
                s.world.map.passable(p) && s.world.map.item_at(p).is_none() && s.world.map.fixture_at(p).is_none()
            })
        })
    };
    let site = (3..40)
        .flat_map(|r| [c.offset(r, 0), c.offset(-r, 0), c.offset(0, r), c.offset(0, -r)])
        .find(|&o| open(&sim, o))
        .expect("open ground");
    (sim, pawn, site)
}

/// How many of `def` lie inside zone `zone`.
fn stored(sim: &Sim, def: rim_sim::defs::DefId, zone: u32) -> u32 {
    sim.world
        .ecs
        .query::<&Thing>()
        .iter()
        .filter(|t| t.def == def && sim.world.zones.at(&sim.world.map, t.pos).is_some_and(|z| z.id == zone))
        .map(|t| t.count)
        .sum()
}

#[test]
fn a_loose_item_is_carried_into_a_stockpile_that_takes_it() {
    let (mut sim, pawn, site) = colony();
    let wood = sim.world.defs.thing_id("wood").unwrap();
    let from = sim.world.pawn_pos(pawn).unwrap().offset(-2, -2);
    sim.world.place_item(wood, from, 30);
    sim.push(Command::Stockpile { a: site, b: site.offset(2, 2), zone: None });
    let mut hauled = false;
    for _ in 0..3_000 {
        sim.step();
        hauled |= matches!(sim.world.ecs.get::<&Pawn>(pawn).unwrap().job, Job::Haul { .. });
    }
    assert!(hauled, "the colonist hauled");
    assert_eq!(stored(&sim, wood, 1), 30, "all the wood is in the stockpile");
}

#[test]
fn an_item_a_stockpile_refuses_is_carried_to_one_that_takes_it() {
    let (mut sim, _, site) = colony();
    let stone = sim.world.defs.thing_id("stone").unwrap();
    // Zone 1 refuses stone; zone 2, further off, takes it.
    sim.push(Command::Stockpile { a: site, b: site, zone: None });
    sim.push(Command::ZoneAllow { zone: 1, thing: stone, on: false });
    sim.push(Command::Stockpile { a: site.offset(2, 2), b: site.offset(2, 2), zone: None });
    sim.step();
    sim.world.put_lot(rim_sim::world::Lot::new(stone, 10), site);
    for _ in 0..3_000 {
        sim.step();
    }
    assert_eq!(stored(&sim, stone, 1), 0, "moved out of the zone that refuses it");
    assert_eq!(stored(&sim, stone, 2), 10, "into the one that takes it");
}

#[test]
fn haul_at_zero_is_never_chosen_and_building_comes_first() {
    let (mut sim, pawn, site) = colony();
    let defs = sim.world.defs.clone();
    let haul = defs.lookup("work_type", "core:haul").unwrap();
    let wood = defs.thing_id("wood").unwrap();
    sim.push(Command::SetPriority { pawn, work: haul, level: 0 });
    sim.push(Command::Stockpile { a: site, b: site.offset(2, 2), zone: None });
    sim.world.place_item(wood, sim.world.pawn_pos(pawn).unwrap().offset(-2, -2), 20);
    for _ in 0..2_000 {
        sim.step();
        assert!(!matches!(sim.world.ecs.get::<&Pawn>(pawn).unwrap().job, Job::Haul { .. }), "no hauling at 0");
    }

    // Build 1, Haul 2: the wall comes first while it waits.
    let build = defs.lookup("work_type", "core:build").unwrap();
    sim.push(Command::SetPriority { pawn, work: build, level: 1 });
    sim.push(Command::SetPriority { pawn, work: haul, level: 2 });
    let c = sim.world.pawn_pos(pawn).unwrap();
    let wall = defs.thing_id("wall").unwrap();
    sim.push(Command::Build { thing: wall, stuff: Some(wood), a: c.offset(1, 3), b: c.offset(1, 3) });
    for _ in 0..3_000 {
        sim.step();
        let hauling = matches!(sim.world.ecs.get::<&Pawn>(pawn).unwrap().job, Job::Haul { .. });
        let waiting = sim.world.ecs.query::<&Blueprint>().iter().count() > 0;
        assert!(!(hauling && waiting), "no hauling while the wall waits");
    }
}

/// A stockpile cell with a wall planned on it has no room: a hauled stack
/// would be walled in and never reached again.
#[test]
fn nothing_is_hauled_onto_a_planned_wall() {
    let (mut sim, pawn, site) = colony();
    let defs = sim.world.defs.clone();
    let (wood, wall) = (defs.thing_id("wood").unwrap(), defs.thing_id("wall").unwrap());
    let build = defs.lookup("work_type", "core:build").unwrap();
    sim.push(Command::SetPriority { pawn, work: build, level: 2 });
    sim.push(Command::Stockpile { a: site, b: site, zone: None });
    sim.push(Command::Build { thing: wall, stuff: Some(wood), a: site, b: site });
    sim.world.place_item(wood, sim.world.pawn_pos(pawn).unwrap().offset(-2, -2), 10);
    sim.step();
    assert_eq!(sim.world.room_for(wood, None, site), 0);
    for _ in 0..6_000 {
        sim.step();
    }
    assert!(sim.world.map.item_at(site).is_none(), "no stack under the wall");
}
