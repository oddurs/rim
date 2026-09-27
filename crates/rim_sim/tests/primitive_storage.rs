//! The stone age's stores (DESIGN.md §4f): baskets, storage pots, the
//! woodpile and the stone bin each take what they should and refuse the rest.

mod common;

use rim_sim::defs::DefId;
use rim_sim::world::{Lot, Store};
use rim_sim::{IVec, Sim};

fn open_cell(sim: &Sim, size: i32) -> IVec {
    let c = sim.world.colony_center().unwrap();
    (4..40)
        .flat_map(|r| [c.offset(r, 0), c.offset(-r, 0), c.offset(0, r), c.offset(0, -r)])
        .find(|&o| {
            (0..size).all(|dx| {
                let p = o.offset(dx, 0);
                sim.world.map.passable(p) && sim.world.map.item_at(p).is_none() && sim.world.map.fixture_at(p).is_none()
            })
        })
        .expect("open ground")
}

/// What each store takes of these things, by how much of a full put went in.
fn takes(id: &str, size: i32, things: &[&str]) -> Vec<(String, u32)> {
    let mut sim = Sim::new(&common::mods(), 3).unwrap();
    let def = sim.world.defs.thing_id(id).unwrap_or_else(|| panic!("no {id}"));
    let at = open_cell(&sim, size);
    let e = sim.world.spawn_fixture_of(def, at, false, None).expect("placed");
    assert!(sim.world.ecs.get::<&Store>(e).is_ok(), "{id} has slots");
    things
        .iter()
        .map(|t| {
            let d: DefId = sim.world.defs.thing_id(t).unwrap_or_else(|| panic!("no {t}"));
            let left = sim.world.put_in_store(e, Lot::new(d, 5));
            let can = sim.world.store_keeps(e, d, None, None);
            (t.to_string(), if can { 5 - left } else { 0 })
        })
        .collect()
}

fn names(v: &[(String, u32)]) -> Vec<&str> {
    v.iter().filter(|(_, n)| *n > 0).map(|(t, _)| t.as_str()).collect()
}

#[test]
fn a_basket_takes_small_things_and_no_bulky_ones() {
    let got = takes("basket", 1, &["berries", "flint", "wood", "branches", "stone"]);
    assert_eq!(names(&got), ["berries", "flint"], "two small stacks; nothing bulky");
}

#[test]
fn a_storage_pot_keeps_food_only() {
    let got = takes("storage_pot", 1, &["berries", "flint", "raw_meat"]);
    assert_eq!(names(&got), ["berries"], "one slot of food");
}

#[test]
fn a_woodpile_takes_wood_and_fuel_three_stacks_deep() {
    let mut sim = Sim::new(&common::mods(), 3).unwrap();
    let def = sim.world.defs.thing_id("woodpile").unwrap();
    let at = open_cell(&sim, 2);
    let e = sim.world.spawn_fixture_of(def, at, false, None).unwrap();
    let (wood, flint) = (sim.world.defs.thing_id("wood").unwrap(), sim.world.defs.thing_id("flint").unwrap());
    let limit = sim.world.defs.thing(wood).stack_limit;
    assert_eq!(sim.world.store_room(e, wood, None), 2 * 3 * limit, "two slots, three stacks deep");
    assert!(sim.world.store_keeps(e, wood, None, None) && !sim.world.store_keeps(e, flint, None, None));
    let branches = sim.world.defs.thing_id("branches").unwrap();
    assert!(sim.world.store_keeps(e, branches, None, None), "branches are fuel");
}

#[test]
fn a_stone_bin_takes_mineral_things() {
    let got = takes("stone_bin", 1, &["stone", "stones", "flint", "wood"]);
    assert_eq!(names(&got), ["stone", "stones"]);
}
