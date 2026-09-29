//! Hauling: colonists carry loose items to the stockpiles that take them.

mod common;

use rim_sim::hecs::Entity;
use rim_sim::world::{Blueprint, Job, Pawn, Thing};
use rim_sim::{Command, IVec, Sim};

fn colony() -> (Sim, Entity, IVec) {
    common::hauling_colony(5)
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
    sim.push(Command::Build { thing: wall, stuff: Some(wood), a: c.offset(1, 3), b: c.offset(1, 3), facing: 0 });
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
    sim.push(Command::Build { thing: wall, stuff: Some(wood), a: site, b: site, facing: 0 });
    sim.world.place_item(wood, sim.world.pawn_pos(pawn).unwrap().offset(-2, -2), 10);
    sim.step();
    assert_eq!(sim.world.room_for(wood, None, site), 0);
    for _ in 0..6_000 {
        sim.step();
    }
    assert!(sim.world.map.item_at(site).is_none(), "no stack under the wall");
}

/// A wall planned over a stack doesn't bury it: the stack moves off before
/// the wall goes up, and the rest of the row still gets its wood (ff97096d).
#[test]
fn a_wall_planned_over_a_stack_moves_it_rather_than_burying_it() {
    let (mut sim, pawn, site) = colony();
    let defs = sim.world.defs.clone();
    let (wood, wall) = (defs.thing_id("wood").unwrap(), defs.thing_id("wall").unwrap());
    let build = defs.lookup("work_type", "core:build").unwrap();
    sim.push(Command::SetPriority { pawn, work: build, level: 1 });
    // All the row's wood lies on its middle cell.
    let row: Vec<IVec> = (0..3).map(|x| site.offset(x, 1)).collect();
    assert_eq!(sim.world.put_lot(rim_sim::world::Lot::new(wood, 40), row[1]), 0);
    sim.push(Command::Build { thing: wall, stuff: Some(wood), a: row[0], b: row[2], facing: 0 });
    for _ in 0..12_000 {
        sim.step();
        for &p in &row {
            let walled = sim.world.map.fixture_at(p).is_some_and(|f| sim.world.ecs.get::<&Blueprint>(f).is_err());
            assert!(!(walled && sim.world.map.item_at(p).is_some()), "a wall stands over a stack at {p:?}");
        }
    }
    assert_eq!(sim.world.ecs.query::<&Blueprint>().iter().count(), 0, "the whole row is built");
}
