//! Cooking: recipes at a fire (mods/crafting), and the stone age's stew,
//! boiled in a pot the cook holds (mods/primitive).

mod common;

use rim_sim::data::{Data, Key};
use rim_sim::hecs::Entity;
use rim_sim::world::{Pawn, Thing, NEED_MAX};
use rim_sim::{Command, IVec, Sim};
use std::path::Path;

/// One colonist in Hand, fed, and a finished campfire beside them.
fn kitchen(dir: &Path) -> (Sim, Entity, Entity) {
    let mut s = Sim::new(dir, 3).unwrap_or_else(|e| panic!("mods load: {e}"));
    s.step();
    let cook = s.world.colonists().next().expect("a founder");
    for e in s.world.pawns.clone() {
        if e != cook {
            let _ = s.world.ecs.despawn(e);
        }
    }
    s.world.pawns.retain(|&e| e == cook);
    common::hands(&mut s);
    // Fed and rested, so the meat goes on the fire rather than down raw.
    s.world.ecs.get::<&mut Pawn>(cook).unwrap().needs.iter_mut().for_each(|n| n.1 = NEED_MAX);
    let home = s.world.pawn_pos(cook).unwrap();
    let open =
        |p: IVec| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none();
    let at = (2..10)
        .flat_map(|d| [home.offset(d, 0), home.offset(-d, 0), home.offset(0, d), home.offset(0, -d)])
        .find(|&p| open(p) && [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().all(|&(dx, dy)| open(p.offset(dx, dy))))
        .expect("room for a fire");
    let fire = s.world.defs.thing_id("core:campfire").unwrap();
    let e = s.world.spawn_fixture_of(fire, at, false, None).expect("placed");
    rim_sim::ai::complete_building(&mut s.world, e);
    (s, cook, e)
}

fn put(s: &mut Sim, id: &str, n: u32, near: IVec) {
    let d = s.world.defs.thing_id(id).unwrap();
    s.world.place_item(d, near, n);
}

fn count(s: &Sim, thing: &str) -> u32 {
    let d = s.world.defs.thing_id(thing).unwrap();
    s.world.ecs.query::<&Thing>().iter().filter(|t| t.def == d).map(|t| t.count).sum()
}

fn nutrition(s: &Sim, thing: &str) -> f64 {
    let d = s.world.defs.thing_id(thing).unwrap();
    s.world.defs.thing(d).food.as_ref().unwrap().nutrition
}

fn bill(s: &mut Sim, fire: Entity, recipe: &str) {
    let site = Data::Int(fire.to_bits().get() as i64);
    let pairs = [("site", site), ("recipe", Data::Str(recipe.into()))];
    let data = Data::Table(pairs.into_iter().map(|(k, v)| (Key::Str(k.into()), v)).collect());
    s.push(Command::ModEvent { name: "crafting:add_bill".into(), data: Some(data) });
    s.step();
}

fn run_until(s: &mut Sim, ticks: u32, done: impl Fn(&Sim) -> bool) -> bool {
    (0..ticks).any(|_| {
        s.step();
        done(s)
    })
}

#[test]
fn a_colonist_roasts_meat_at_the_campfire_and_it_feeds_better_than_raw() {
    let dir = common::test_mods("cook-roast", &["core", "crafting"], &[]);
    let (mut s, cook, fire) = kitchen(&dir);
    let tags = &s.world.defs.thing(s.world.defs.thing_id("core:campfire").unwrap()).tags;
    assert!(tags.iter().any(|t| t == "crafting:fire"), "crafting makes the campfire a station");
    let raw_before = count(&s, "core:raw_meat");
    let home = s.world.pawn_pos(cook).unwrap();
    put(&mut s, "core:raw_meat", 4, home);
    bill(&mut s, fire, "crafting:roast_meat");
    assert!(run_until(&mut s, 12_000, |s| count(s, "crafting:roast_meat") >= 1), "the meat is roasted");
    assert_eq!(count(&s, "core:raw_meat"), raw_before + 2, "out of two raw cuts");
    let cook_work = s.world.defs.lookup("work_type", "crafting:cook").expect("a Cook column");
    assert_eq!(s.world.defs.work_types[cook_work as usize].auto.weight, 3, "weighed as food work");
    // One roast feeds more than the two cuts it took.
    assert!(nutrition(&s, "crafting:roast_meat") > 2.0 * nutrition(&s, "core:raw_meat"));
}

#[test]
fn a_stew_boils_in_a_pot_the_cook_holds_and_the_pot_is_kept() {
    let dir = common::test_mods("cook-stew", &["core", "crafting", "primitive"], &[]);
    let (mut s, cook, fire) = kitchen(&dir);
    let home = s.world.pawn_pos(cook).unwrap();
    let pots = count(&s, "primitive:pot");
    put(&mut s, "core:raw_meat", 1, home);
    put(&mut s, "core:berries", 2, home);
    bill(&mut s, fire, "primitive:stew");
    // No pot, no stew: the bill waits for the vessel.
    for _ in 0..1_500 {
        s.step();
    }
    assert_eq!(count(&s, "primitive:stew"), 0, "nothing boils without a pot");
    put(&mut s, "primitive:pot", 1, home);
    assert!(run_until(&mut s, 12_000, |s| count(s, "primitive:stew") == 2), "two bowls of stew");
    assert_eq!(count(&s, "primitive:pot"), pots + 1, "the pot isn't used up");
    let raw = nutrition(&s, "core:raw_meat") + 2.0 * nutrition(&s, "core:berries");
    assert!(2.0 * nutrition(&s, "primitive:stew") > raw, "the stew feeds more than what went in");
}
