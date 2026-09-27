//! Store levels (DESIGN.md §4f): a stack moves only to a store at a higher
//! level that takes it, nearest within that level, and the haul search
//! answers from the store index.

mod common;

use rim_sim::ai::{haul_plan, HaulPlan};
use rim_sim::defs::DefId;
use rim_sim::filter::FilterEdit;
use rim_sim::hecs::Entity;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{Faction, Job, Lot, Pawn, Thing};
use rim_sim::zone::StoreRef;
use rim_sim::{Command, IVec, Sim};

const NORMAL: u8 = 1;
const PREFERRED: u8 = 2;

/// Units of `def` lying in zone `zone`.
fn in_zone(sim: &Sim, def: DefId, zone: u32) -> u32 {
    sim.world
        .ecs
        .query::<&Thing>()
        .iter()
        .filter(|t| t.def == def && sim.world.zones.at(&sim.world.map, t.pos).is_some_and(|z| z.id == zone))
        .map(|t| t.count)
        .sum()
}

fn run(sim: &mut Sim, ticks: u32) {
    for _ in 0..ticks {
        sim.step();
    }
}

#[test]
fn a_stack_climbs_to_a_preferred_store_and_never_trades_between_equals() {
    let (mut sim, _, site) = common::hauling_colony(5);
    let wood = sim.world.defs.thing_id("wood").unwrap();
    // Two Normal zones, the wood in the first.
    sim.push(Command::Stockpile { a: site, b: site, zone: None });
    sim.push(Command::Stockpile { a: site.offset(2, 2), b: site.offset(2, 2), zone: None });
    sim.step();
    sim.world.put_lot(Lot::new(wood, 20), site);
    run(&mut sim, 2_000);
    assert_eq!(in_zone(&sim, wood, 1), 20, "two Normal zones never trade");
    let e = sim.world.map.item_at(site).unwrap();
    assert_eq!(haul_plan(&sim.world, e), Some(HaulPlan::Stays { level: NORMAL }));
    // Raise the second: the wood climbs.
    sim.push(Command::StoreLevel { store: StoreRef::Zone(2), level: PREFERRED });
    run(&mut sim, 2_000);
    assert_eq!(in_zone(&sim, wood, 2), 20, "it climbed to Preferred");
    assert_eq!(in_zone(&sim, wood, 1), 0);
}

#[test]
fn a_store_that_stops_taking_a_thing_lets_it_go_to_any_level() {
    let (mut sim, _, site) = common::hauling_colony(5);
    let stone = sim.world.defs.thing_id("stone").unwrap();
    sim.push(Command::Stockpile { a: site, b: site, zone: None });
    sim.push(Command::StoreLevel { store: StoreRef::Zone(1), level: PREFERRED });
    sim.push(Command::Stockpile { a: site.offset(2, 2), b: site.offset(2, 2), zone: None });
    sim.push(Command::StoreLevel { store: StoreRef::Zone(2), level: 0 });
    sim.step();
    sim.world.put_lot(Lot::new(stone, 10), site);
    run(&mut sim, 500);
    assert_eq!(in_zone(&sim, stone, 1), 10, "kept where it is");
    sim.push(Command::StoreFilter { store: StoreRef::Zone(1), edit: FilterEdit::Thing { thing: stone, on: false } });
    run(&mut sim, 2_000);
    assert_eq!(in_zone(&sim, stone, 2), 10, "down to a Low store, since the Preferred one let it go");
}

#[test]
fn two_haulers_never_fill_the_same_last_cell() {
    let (mut sim, pawn, site) = common::hauling_colony(5);
    let defs = sim.world.defs.clone();
    let (wood, stone) = (defs.thing_id("wood").unwrap(), defs.thing_id("stone").unwrap());
    let human = defs.creature_id("human").unwrap();
    let at = sim.world.pawn_pos(pawn).unwrap();
    let other = sim.world.spawn_pawn(human, Faction::Player, at.offset(1, 0), None);
    for (w, d) in defs.work_types.iter().enumerate() {
        let level = if d.id == "core:haul" { 1 } else { 0 };
        sim.push(Command::SetPriority { pawn: other, work: w as DefId, level });
    }
    sim.push(Command::Stockpile { a: site, b: site, zone: None });
    sim.step();
    let cells = common::loose_cells(&sim, at, 2);
    sim.world.put_lot(Lot::new(wood, 10), cells[0]);
    sim.world.put_lot(Lot::new(stone, 10), cells[1]);
    let mut both_bound = false;
    for _ in 0..1_500 {
        sim.step();
        let bound: Vec<IVec> = [pawn, other]
            .iter()
            .filter_map(|&p| match sim.world.ecs.get::<&Pawn>(p).unwrap().job {
                Job::Haul { to, .. } => Some(to),
                _ => None,
            })
            .collect();
        both_bound |= bound.len() == 2 && bound[0] == bound[1];
    }
    assert!(!both_bound, "never both bound for the one cell");
    assert_eq!(in_zone(&sim, wood, 1) + in_zone(&sim, stone, 1), 10, "one stack filled it");
    let total = |d| sim.world.stock.on_map(d);
    assert_eq!((total(wood), total(stone)), (10, 10), "nothing lost");
}

#[test]
fn a_hauler_whose_cell_filled_goes_on_to_the_next_best_place() {
    let (mut sim, pawn, site) = common::hauling_colony(5);
    let (wood, stone) = (sim.world.defs.thing_id("wood").unwrap(), sim.world.defs.thing_id("stone").unwrap());
    sim.push(Command::Stockpile { a: site, b: site.offset(3, 0), zone: None });
    sim.step();
    let at = sim.world.pawn_pos(pawn).unwrap();
    sim.world.put_lot(Lot::new(wood, 10), common::loose_cells(&sim, at, 1)[0]);
    // Wait for the hauler to be carrying toward a cell, then fill that cell.
    let mut filled = None;
    for _ in 0..2_000 {
        sim.step();
        let (job, carry) = {
            let p = sim.world.ecs.get::<&Pawn>(pawn).unwrap();
            (p.job.clone(), p.carry)
        };
        if let (Job::Haul { to, stage: 1, .. }, Some(_)) = (job, carry) {
            sim.world.put_lot(Lot::new(stone, 75), to);
            filled = Some(to);
            break;
        }
    }
    let filled = filled.expect("the hauler set off");
    run(&mut sim, 1_500);
    assert_eq!(in_zone(&sim, wood, 1), 10, "the wood went to another cell of the zone");
    assert_ne!(sim.world.map.item_at(filled).and_then(|e| sim.world.thing(e)).map(|t| t.def), Some(wood));
}

#[test]
fn the_store_index_is_what_a_rebuild_says_through_a_working_game() {
    let (mut sim, pawn, site) = common::hauling_colony(9);
    let defs = sim.world.defs.clone();
    let (wood, stone) = (defs.thing_id("wood").unwrap(), defs.thing_id("stone").unwrap());
    let at = sim.world.pawn_pos(pawn).unwrap();
    for i in 0..6 {
        sim.world.place_item(if i % 2 == 0 { wood } else { stone }, at.offset(-3 - i, 3), 30);
    }
    sim.push(Command::Stockpile { a: site, b: site.offset(2, 1), zone: None });
    sim.push(Command::Stockpile { a: site.offset(0, 3), b: site.offset(2, 3), zone: None });
    for step in 0..3_000 {
        if step == 1_000 {
            sim.push(Command::StoreLevel { store: StoreRef::Zone(2), level: PREFERRED });
        }
        sim.step();
        if step % 200 == 0 {
            let kept = sim.world.stores.clone();
            sim.world.rebuild_stores();
            assert_eq!(kept, sim.world.stores, "at step {step}");
        }
    }
}

#[test]
fn levels_survive_a_save_and_old_zones_load_at_normal() {
    let (mut sim, _, site) = common::hauling_colony(5);
    sim.push(Command::Stockpile { a: site, b: site.offset(1, 1), zone: None });
    sim.push(Command::StoreLevel { store: StoreRef::Zone(1), level: 3 });
    sim.push(Command::StoreLevel { store: StoreRef::Zone(1), level: 200 });
    sim.step();
    assert_eq!(sim.world.zones.list[0].level, 4, "clamped to the top of the scale");
    let back = Snapshot::capture(&sim).restore(&common::mods(), &|_| true).unwrap();
    assert_eq!(back.world.zones, sim.world.zones);
    assert_eq!(back.world.stores, sim.world.stores);
    let old: rim_sim::zone::Zone = serde_json::from_str(r#"{"id":3,"name":"Stockpile 3","allows":[2,1]}"#).unwrap();
    assert_eq!(old.level, NORMAL);
}

#[test]
fn a_loose_stack_with_nowhere_to_go_waits() {
    let (mut sim, pawn, site) = common::hauling_colony(5);
    let (wood, stone) = (sim.world.defs.thing_id("wood").unwrap(), sim.world.defs.thing_id("stone").unwrap());
    sim.push(Command::Stockpile { a: site, b: site, zone: None });
    sim.push(Command::StoreFilter { store: StoreRef::Zone(1), edit: FilterEdit::Thing { thing: wood, on: false } });
    sim.step();
    sim.world.put_lot(Lot::new(stone, 75), site);
    let at = common::loose_cells(&sim, sim.world.pawn_pos(pawn).unwrap(), 1)[0];
    sim.world.put_lot(Lot::new(wood, 5), at);
    let e: Entity = sim.world.map.item_at(at).unwrap();
    assert_eq!(haul_plan(&sim.world, e), Some(HaulPlan::Waits));
}

/// The store inspector's Accepts tab sends a category edit: the stockpile
/// that stops taking materials lets its stone go to one that does.
#[test]
fn a_category_edit_from_the_inspector_changes_where_hauls_go() {
    let (mut sim, _, site) = common::hauling_colony(5);
    let stone = sim.world.defs.thing_id("stone").unwrap();
    let materials = sim.world.defs.lookup("item_category", "core:materials").unwrap();
    sim.push(Command::Stockpile { a: site, b: site, zone: None });
    sim.push(Command::Stockpile { a: site.offset(2, 2), b: site.offset(2, 2), zone: None });
    sim.step();
    sim.world.put_lot(Lot::new(stone, 10), site);
    run(&mut sim, 300);
    assert_eq!(in_zone(&sim, stone, 1), 10);
    sim.push(Command::StoreFilter {
        store: StoreRef::Zone(1),
        edit: FilterEdit::Category { category: materials, on: false },
    });
    run(&mut sim, 2_000);
    assert_eq!((in_zone(&sim, stone, 1), in_zone(&sim, stone, 2)), (0, 10), "stone moved to the zone that takes it");
}
