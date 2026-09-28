//! Farming: a growing zone is sown while the season allows, grows with
//! the weather, and is harvested when grown (mods/farming).

mod common;

use rim_sim::systems::{self, PLANT_PASS};
use rim_sim::world::{Blueprint, Growth, GROWN};
use rim_sim::{Command, IVec, Sim, TICKS_PER_DAY};

/// Potatoes in a third of a day, so a test sees a whole season's work.
const QUICK: &str = r##"
[[patch]]
target = "thing/farming:potato_plant"
set = { grow = { days = 0.3 } }
"##;

fn sim(name: &str) -> Sim {
    let dir = common::test_mods(name, &["core", "weather", "farming"], &[("quick", &[("defs/quick.toml", QUICK)])]);
    let mut s = Sim::new(&dir, 6).expect("mods load");
    common::hands(&mut s);
    s
}

fn pin(s: &mut Sim, id: &str, v: f64) {
    let f = s.world.defs.lookup("field", id).unwrap() as usize;
    s.world.fields.set_ambient(f, Some(v));
}

/// A mild, bright day that holds.
fn summer(s: &mut Sim) {
    pin(s, "core:temperature", 20.0);
    pin(s, "core:light", 100.0);
}

/// A clear square of a terrain beside the colony, `n` on a side.
fn plot(s: &mut Sim, terrain: &str, at: IVec, n: i32) -> (IVec, IVec) {
    let t = s.world.defs.lookup("terrain", terrain).unwrap();
    for y in 0..n {
        for x in 0..n {
            let p = at.offset(x, y);
            if let Some(f) = s.world.map.fixture_at(p) {
                s.world.despawn_thing(f);
            }
            if let Some(i) = s.world.map.item_at(p) {
                s.world.despawn_thing(i);
            }
            s.world.map.set_terrain(p, t, 100);
        }
    }
    (at, at.offset(n - 1, n - 1))
}

fn things(s: &Sim, id: &str, a: IVec, b: IVec) -> Vec<rim_sim::hecs::Entity> {
    let def = s.world.defs.thing_id(id).unwrap();
    let m = &s.world.map;
    let mut out = Vec::new();
    for y in a.y..=b.y {
        for x in a.x..=b.x {
            let p = IVec::new(x, y);
            for e in [m.fixture_at(p), m.item_at(p)].into_iter().flatten() {
                if s.world.thing(e).is_some_and(|t| t.def == def) && !out.contains(&e) {
                    out.push(e);
                }
            }
        }
    }
    out
}

fn run(s: &mut Sim, ticks: u64, done: impl Fn(&Sim) -> bool) -> bool {
    for _ in 0..ticks {
        s.step();
        if done(s) {
            return true;
        }
    }
    false
}

#[test]
fn a_field_is_sown_grown_and_harvested() {
    let mut s = sim("farm-season");
    summer(&mut s);
    let potato = s.world.defs.thing_id("farming:potato_plant").unwrap();
    let at = s.world.colony_center().unwrap().offset(4, 4);
    let (a, b) = plot(&mut s, "rich_soil", at, 3);
    s.push(Command::GrowZone { a, b, zone: None, plant: potato });
    s.step();
    let z = s.world.zones.at(&s.world.map, a).expect("a growing zone").clone();
    assert_eq!(z.plant, Some(potato));
    // It takes no items, so nothing is hauled to it.
    let berries = s.world.defs.thing_id("berries").unwrap();
    assert!(!z.takes(berries));

    // Within a pass the empty cells are laid out; colonists sow them.
    assert!(run(&mut s, 2 * PLANT_PASS, |s| things(s, "farming:potato_plant", a, b).len() == 9), "laid out");
    let sown =
        |s: &Sim| things(s, "farming:potato_plant", a, b).iter().all(|&e| s.world.ecs.get::<&Blueprint>(e).is_err());
    assert!(run(&mut s, TICKS_PER_DAY / 2, sown), "sown");
    // They grow, are marked when grown, and are taken up: potatoes.
    let dug = |s: &Sim| !things(s, "farming:potatoes", a.offset(-3, -3), b.offset(3, 3)).is_empty();
    assert!(run(&mut s, TICKS_PER_DAY, dug), "harvested");
    // And the field is sown again.
    assert!(
        run(&mut s, TICKS_PER_DAY / 2, |s| {
            things(s, "farming:potato_plant", a, b)
                .iter()
                .any(|&e| s.world.ecs.get::<&Growth>(e).is_ok_and(|g| g.progress < GROWN))
        }),
        "sown again"
    );
}

#[test]
fn nothing_is_sown_out_of_season_and_unstarted_plans_are_taken_back() {
    let mut s = sim("farm-frost");
    let potato = s.world.defs.thing_id("farming:potato_plant").unwrap();
    let at = s.world.colony_center().unwrap().offset(-8, 6);
    let (a, b) = plot(&mut s, "grass", at, 4);
    s.push(Command::GrowZone { a, b, zone: None, plant: potato });
    s.step();
    // Too cold for potatoes: nothing laid out.
    pin(&mut s, "core:temperature", 2.0);
    pin(&mut s, "core:light", 100.0);
    s.world.tick += PLANT_PASS - s.world.tick % PLANT_PASS;
    systems::tend(&mut s.world);
    assert!(things(&s, "farming:potato_plant", a, b).is_empty(), "nothing sown in the cold");
    // Warm: laid out.
    pin(&mut s, "core:temperature", 18.0);
    systems::tend(&mut s.world);
    assert_eq!(things(&s, "farming:potato_plant", a, b).len(), 16);
    // A frost comes before anyone starts: the plans are taken back.
    pin(&mut s, "core:temperature", 1.0);
    systems::tend(&mut s.world);
    assert!(things(&s, "farming:potato_plant", a, b).is_empty(), "plans taken back");
}

/// Fertility from the terrain: the same crop, the same weather, faster on
/// rich soil than on dirt.
#[test]
fn a_crop_grows_faster_on_richer_ground() {
    let mut s = sim("farm-soil");
    summer(&mut s);
    let potato = s.world.defs.thing_id("farming:potato_plant").unwrap();
    let home = s.world.colony_center().unwrap();
    let (rich, _) = plot(&mut s, "rich_soil", home.offset(10, -10), 1);
    let (dirt, _) = plot(&mut s, "dirt", home.offset(12, -10), 1);
    let mut sown = Vec::new();
    for p in [rich, dirt] {
        let e = s.world.spawn_fixture(potato, p, false).unwrap();
        s.world.plant_seedling(e);
        sown.push(e);
    }
    // Only the plant pass: whole cycles, so both are worked out alike.
    for _ in 0..40 {
        s.world.tick += PLANT_PASS;
        systems::grow(&mut s.world);
    }
    let g = |e| s.world.ecs.get::<&Growth>(e).map(|g| g.progress).unwrap_or(GROWN);
    assert!(g(sown[0]) > g(sown[1]), "rich {} dirt {}", g(sown[0]), g(sown[1]));
}

/// A growing zone survives a save, crop and all.
#[test]
fn a_field_keeps_its_crop_across_a_save() {
    let mut s = sim("farm-save");
    let flax = s.world.defs.thing_id("farming:flax_plant").unwrap();
    let at = s.world.colony_center().unwrap().offset(6, -6);
    let (a, b) = plot(&mut s, "grass", at, 2);
    s.push(Command::GrowZone { a, b, zone: None, plant: flax });
    s.step();
    let dir = common::test_mods(
        "farm-save-load",
        &["core", "weather", "farming"],
        &[("quick", &[("defs/quick.toml", QUICK)])],
    );
    let back = rim_sim::snapshot::Snapshot::capture(&s).restore(&dir, &|_| true).expect("loads");
    assert_eq!(back.world.zones.at(&back.world.map, a).and_then(|z| z.plant), Some(flax));
    assert_eq!(back.world.state_hash(), s.world.state_hash());
}
