//! Spoilage (DESIGN.md §4f): food loses condition where it lies, slower in
//! a store that keeps, dry in one that shelters, and rots away at none.

mod common;

use rim_sim::hecs::Entity;
use rim_sim::snapshot::Snapshot;
use rim_sim::systems::{self, SPOIL_PASS};
use rim_sim::world::{Lot, Spoiling};
use rim_sim::{IVec, Sim, TICKS_PER_DAY};

/// A plain box (no keeping, no shelter) and a dry one (shelter only), so
/// shelter is measured apart from a pot's sealing.
const BOXES: &str = r##"
[[thing]]
id = "open_box"
label = "open box"
color = "#8a6a45"
category = "building"
hp = 50
store = { slots = 2 }

[[thing]]
id = "dry_box"
label = "dry box"
color = "#6a5a45"
category = "building"
hp = 50
store = { slots = 2, shelter = true }
"##;

fn sim(name: &str) -> (Sim, std::path::PathBuf) {
    let dir = common::test_mods(name, &["core", "crafting", "primitive"], &[("boxes", &[("defs/boxes.toml", BOXES)])]);
    (Sim::new(&dir, 8).expect("mods load"), dir)
}

fn pin(s: &mut Sim, id: &str, v: f64) {
    let f = s.world.defs.lookup("field", id).unwrap() as usize;
    s.world.fields.set_ambient(f, Some(v));
}

/// Mild and dry, or mild and raining.
fn weather(s: &mut Sim, rain: f64) {
    pin(s, "core:temperature", 10.0);
    pin(s, "core:precipitation", rain);
}

/// Only the spoil pass runs, so nothing eats or moves the berries.
fn spoil_for(s: &mut Sim, days: f64) {
    let passes = (days * TICKS_PER_DAY as f64 / SPOIL_PASS as f64) as u64;
    for _ in 0..passes {
        s.world.tick += SPOIL_PASS;
        systems::spoil(&mut s.world);
    }
}

/// Open ground away from the colony: `n` free cells in a row from `x`.
fn ground(s: &Sim, n: i32) -> IVec {
    let c = s.world.colony_center().unwrap();
    (8..60)
        .flat_map(|r| [c.offset(r, 5), c.offset(-r, -5), c.offset(5, r), c.offset(-5, -r)])
        .find(|&o| {
            (0..n).all(|x| {
                let p = o.offset(x, 0);
                s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none()
            })
        })
        .expect("open ground")
}

fn berries(s: &mut Sim, at: IVec) -> Entity {
    let def = s.world.defs.thing_id("berries").unwrap();
    assert_eq!(s.world.place_item(def, at, 20), 0);
    s.world.map.item_at(at).expect("berries down")
}

/// A container at `at` holding 20 berries; the berries' entity.
fn stored(s: &mut Sim, store: &str, at: IVec) -> Entity {
    let def = s.world.defs.thing_id(store).unwrap();
    let e = s.world.spawn_fixture_of(def, at, false, None).expect("placed");
    let b = s.world.defs.thing_id("berries").unwrap();
    assert_eq!(s.world.put_in_store(e, Lot::new(b, 20)), 0);
    let slots = s.world.ecs.get::<&rim_sim::world::Store>(e).unwrap().slots.clone();
    slots.into_iter().flatten().next().expect("in the store")
}

fn hp(s: &Sim, e: Entity) -> Option<i32> {
    s.world.thing(e).map(|t| t.hp)
}

#[test]
fn berries_in_a_pot_outlast_berries_on_the_ground() {
    let (mut s, _) = sim("spoil-pot");
    weather(&mut s, 0.0);
    let at = ground(&s, 3);
    let (loose, potted) = (berries(&mut s, at), stored(&mut s, "primitive:storage_pot", at.offset(2, 0)));
    let berry = s.world.defs.thing_id("berries").unwrap();
    let before = s.world.stock.on_map(berry);
    spoil_for(&mut s, 3.0);
    let (a, b) = (hp(&s, loose).unwrap(), hp(&s, potted).unwrap());
    assert!(a < b && b < 100, "loose {a}, potted {b}");
    // Six days loose at a rate of 1 is rotten; the pot's keep 2.5 times as long.
    spoil_for(&mut s, 3.5);
    assert!(hp(&s, loose).is_none(), "the loose berries rotted away");
    assert!(s.world.map.item_at(at).is_none());
    assert!(hp(&s, potted).is_some_and(|h| h > 0), "the potted berries keep");
    // Rotting took them off the colony's count, through the ledger.
    assert_eq!(s.world.stock.on_map(berry), before - 20);
}

#[test]
fn a_sheltering_store_keeps_its_contents_dry() {
    let run = |rain: f64| {
        let (mut s, _) = sim(&format!("spoil-rain-{rain}"));
        weather(&mut s, rain);
        let at = ground(&s, 5);
        let loose = berries(&mut s, at);
        let open = stored(&mut s, "boxes:open_box", at.offset(2, 0));
        let dry = stored(&mut s, "boxes:dry_box", at.offset(4, 0));
        spoil_for(&mut s, 2.0);
        (hp(&s, loose).unwrap_or(0), hp(&s, open).unwrap_or(0), hp(&s, dry).unwrap_or(0))
    };
    let (dry_loose, dry_open, dry_dry) = run(0.0);
    let (wet_loose, wet_open, wet_dry) = run(4.0);
    assert!(wet_loose < dry_loose && wet_open < dry_open, "rain spoils what it falls on");
    assert_eq!(wet_dry, dry_dry, "a shelter keeps the rain off");
}

#[test]
fn a_save_keeps_how_far_things_have_spoiled() {
    let (mut s, dir) = sim("spoil-save");
    weather(&mut s, 0.0);
    let at = ground(&s, 1);
    let loose = berries(&mut s, at);
    // Part of a point lost, to see the remainder kept too.
    s.world.tick += SPOIL_PASS - s.world.tick % SPOIL_PASS;
    for _ in 0..(3 * 4 + 1) {
        s.world.tick += SPOIL_PASS;
        systems::spoil(&mut s.world);
    }
    let lost = s.world.ecs.get::<&Spoiling>(loose).map(|l| *l).expect("spoiling");
    let back = Snapshot::capture(&s).restore(&dir, &|_| true).expect("loads");
    let again = back.world.map.item_at(at).expect("still there");
    assert_eq!(back.world.thing(again).map(|t| t.hp), hp(&s, loose));
    assert_eq!(back.world.ecs.get::<&Spoiling>(again).map(|l| *l).ok(), Some(lost));
    assert_eq!(back.world.state_hash(), s.world.state_hash());
}

#[test]
fn a_spoil_pass_over_thousands_of_stacks() {
    let (mut s, _) = sim("spoil-cost");
    weather(&mut s, 1.0);
    let berry = s.world.defs.thing_id("berries").unwrap();
    let m = &s.world.map;
    let open: Vec<IVec> = (0..m.plane())
        .map(|i| m.pos(i))
        .filter(|&p| m.passable(p) && m.fixture_at(p).is_none() && m.item_at(p).is_none())
        .take(4000)
        .collect();
    for p in open {
        s.world.place_item(berry, p, 5);
    }
    let stacks = s.world.stock.on_map(berry) / 5;
    let passes = 40;
    let mut fastest = f64::MAX;
    for _ in 0..3 {
        let t0 = std::time::Instant::now();
        for _ in 0..passes {
            s.world.tick += SPOIL_PASS;
            systems::spoil(&mut s.world);
        }
        fastest = fastest.min(t0.elapsed().as_secs_f64() * 1e3 / passes as f64);
    }
    println!("spoil pass, about {stacks} stacks: {fastest:.3} ms a pass, {:.4} ms a tick", fastest / SPOIL_PASS as f64);
    // The bound catches a pass that stopped staggering, with room for a
    // machine running every test at once.
    let slack = if std::env::var_os("CI").is_some() { 6.0 } else { 1.0 };
    assert!(fastest <= 3.0 * slack, "{fastest:.3} ms a pass");
}
