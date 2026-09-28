//! Plants grow with the weather (DESIGN.md §4c): by their `grow` terms at
//! their cell, dormant in the cold, killed by what harms them, and spreading
//! where they would grow well.

mod common;

use rim_sim::hecs::Entity;
use rim_sim::systems::{self, GROW_EVERY, PLANT_PASS};
use rim_sim::world::{Growth, GROWN};
use rim_sim::{IVec, Sim, TICKS_PER_DAY};

/// A sprout that grows by the warmth, a pinned moisture, the light and the
/// soil, all declared; frost below -2° harms it.
const GARDEN: &str = r##"
[[field]]
id = "moist"
label = "moisture"
unit = "%"
range = [0.0, 100.0]
color_low = "#000000"
color_high = "#ffffff"

[[thing]]
id = "sprout"
label = "sprout"
color = "#5fa050"
category = "plant"
natural = true
spawn = { terrain = ["core:grass", "core:sand", "core:rich_soil"], density = 0.02, spread = true }

[thing.grow]
days = 1.0

[thing.grow.rate.growth]
of = [
  { field = "core:temperature", curve = [[0, 0.0], [20, 1.0]] },
  { field = "moist", curve = [[0, 0.0], [100, 1.0]] },
  { field = "core:light", curve = [[0, 0.0], [100, 1.0]] },
  { terrain = "fertility" },
]

[thing.grow.harm.frost]
of = [{ field = "core:temperature", curve = [[-5, 3.0], [-2, 0.0]] }]
"##;

/// Wild spread on the weather plugin's ground: the same plant, reading
/// wetness instead of a pinned moisture.
const MEADOW: &str = r##"
[[thing]]
id = "herb"
label = "herb"
color = "#5fa050"
category = "plant"
natural = true
spawn = { terrain = ["core:grass", "core:sand", "core:rich_soil", "core:dirt"], density = 0.03, spread = true }

[thing.grow]
days = 4.0

[thing.grow.rate.growth]
of = [{ field = "weather:wetness", curve = [[0, 0.0], [60, 1.0]] }, { terrain = "fertility" }]
"##;

fn garden(name: &str) -> Sim {
    let dir = common::test_mods(name, &["core"], &[("garden", &[("defs/garden.toml", GARDEN)])]);
    Sim::new(&dir, 11).expect("mods load")
}

fn pin(s: &mut Sim, id: &str, v: f64) {
    let f = s.world.defs.lookup("field", id).unwrap() as usize;
    s.world.fields.set_ambient(f, Some(v));
}

/// Warm, bright and half-moist: the sprout's rate is half its fertility.
fn fair_weather(s: &mut Sim) {
    pin(s, "core:temperature", 20.0);
    pin(s, "garden:moist", 50.0);
    pin(s, "core:light", 100.0);
}

/// Only the plant pass runs: a day takes a moment.
fn grow_for(s: &mut Sim, days: f64) {
    for _ in 0..(days * TICKS_PER_DAY as f64) as u64 {
        s.world.tick += 1;
        if s.world.tick.is_multiple_of(PLANT_PASS) {
            systems::grow(&mut s.world);
        }
    }
}

fn growth(s: &Sim, e: Entity) -> Growth {
    *s.world.ecs.get::<&Growth>(e).expect("growing")
}

/// An open cell of a terrain, cleared of anything on it.
fn plot(s: &mut Sim, terrain: &str, n: usize) -> IVec {
    let t = s.world.defs.lookup("terrain", terrain).unwrap();
    let home = s.world.colony_center().unwrap();
    let m = &s.world.map;
    (0..m.plane())
        .map(|i| m.pos(i))
        .filter(|&p| p.chebyshev(home) > 12 && m.terrain[m.idx(p)] == t && m.fixture_at(p).is_none())
        .nth(n)
        .unwrap_or_else(|| panic!("no {terrain} cell"))
}

fn sow(s: &mut Sim, thing: &str, p: IVec) -> Entity {
    let def = s.world.defs.thing_id(thing).unwrap();
    let e = s.world.spawn_fixture(def, p, false).expect("sown");
    s.world.plant_seedling(e);
    e
}

#[test]
fn growth_follows_the_weather_and_the_soil_as_declared() {
    let mut s = garden("grow-rate");
    fair_weather(&mut s);
    let (g, r) = (plot(&mut s, "grass", 0), plot(&mut s, "rich_soil", 0));
    let (on_grass, on_rich) = (sow(&mut s, "garden:sprout", g), sow(&mut s, "garden:sprout", r));
    // Whole cycles, so every plant has been worked out the same number of
    // times: rate 0.5 on grass (fertility 1), 0.7 on rich soil (1.4).
    let cycles = 10;
    grow_for(&mut s, (cycles * PLANT_PASS * GROW_EVERY) as f64 / TICKS_PER_DAY as f64);
    let per_cycle = (PLANT_PASS * GROW_EVERY) as f64 / TICKS_PER_DAY as f64 * GROWN as f64;
    assert_eq!(growth(&s, on_grass).progress as f64, (0.5 * per_cycle * cycles as f64).round());
    assert_eq!(growth(&s, on_rich).progress as f64, (0.7 * per_cycle * cycles as f64).round());
    // Each input scales it: half the light, half the growth.
    let before = growth(&s, on_grass).progress;
    pin(&mut s, "core:light", 50.0);
    grow_for(&mut s, (cycles * PLANT_PASS * GROW_EVERY) as f64 / TICKS_PER_DAY as f64);
    assert_eq!((growth(&s, on_grass).progress - before) as f64, (0.25 * per_cycle * cycles as f64).round());
    // In the dark it stops, and says so.
    pin(&mut s, "core:light", 0.0);
    let held = growth(&s, on_grass).progress;
    grow_for(&mut s, 0.5);
    assert_eq!(growth(&s, on_grass).progress, held);
    assert!(growth(&s, on_grass).dormant);
}

#[test]
fn a_hard_frost_kills_a_tender_plant() {
    let mut s = garden("grow-frost");
    fair_weather(&mut s);
    let p = plot(&mut s, "grass", 3);
    let e = sow(&mut s, "garden:sprout", p);
    // A light frost holds it without harm; a hard one kills it within a day.
    pin(&mut s, "core:temperature", -1.0);
    grow_for(&mut s, 1.0);
    assert_eq!(growth(&s, e).health, GROWN);
    pin(&mut s, "core:temperature", -8.0);
    grow_for(&mut s, 0.5);
    assert!(s.world.thing(e).is_none(), "killed by the frost");
    assert!(s.world.map.fixture_at(p).is_none());
}

#[test]
fn berry_bushes_bear_again_only_when_it_is_warm() {
    let mut s = garden("grow-bush");
    let p = plot(&mut s, "grass", 5);
    let bush = sow(&mut s, "core:berry_bush", p);
    let harvest = s.world.defs.thing(s.world.defs.thing_id("berry_bush").unwrap()).harvest[0].key();
    assert!(!s.world.harvest_ready(bush, harvest), "a seedling has no berries");
    // Winter: cold, short grey days. Nothing grows for a week.
    pin(&mut s, "core:temperature", 1.0);
    pin(&mut s, "core:light", 40.0);
    grow_for(&mut s, 7.0);
    assert!(!s.world.harvest_ready(bush, harvest) && growth(&s, bush).dormant, "no berries in winter");
    // Summer: berries within a few days.
    pin(&mut s, "core:temperature", 20.0);
    pin(&mut s, "core:light", 100.0);
    grow_for(&mut s, 4.0);
    assert!(s.world.harvest_ready(bush, harvest), "berries by summer: {:?}", growth(&s, bush));
}

#[test]
fn wild_plants_spread_more_on_wet_fertile_ground_than_dry_sand() {
    let dir = common::test_mods("grow-spread", &["core", "weather"], &[("meadow", &[("defs/meadow.toml", MEADOW)])]);
    let mut s = Sim::new(&dir, 4).expect("mods load");
    s.step();
    let herb = s.world.defs.thing_id("meadow:herb").unwrap();
    let count = |s: &Sim, terrain: &str| {
        let t = s.world.defs.lookup("terrain", terrain).unwrap();
        let m = &s.world.map;
        let cells = (0..m.plane()).filter(|&i| m.terrain[i] == t).count();
        let herbs = (0..m.plane())
            .filter(|&i| {
                m.terrain[i] == t && m.fixture[i].is_some_and(|e| s.world.thing(e).is_some_and(|x| x.def == herb))
            })
            .count();
        (herbs, cells)
    };
    let (rich0, sand0) = (count(&s, "rich_soil").0, count(&s, "sand").0);
    // A year of the spread pass, with the ground at its own dampness.
    let passes = s.world.defs.calendar.year_days as u64 * TICKS_PER_DAY / 500;
    for _ in 0..passes {
        systems::spread_plants(&mut s.world);
    }
    let ((rich, rich_cells), (sand, sand_cells)) = (count(&s, "rich_soil"), count(&s, "sand"));
    let (per_rich, per_sand) =
        ((rich - rich0) as f64 / rich_cells as f64, (sand - sand0) as f64 / sand_cells.max(1) as f64);
    println!("a year's spread: rich soil {per_rich:.4} a cell, sand {per_sand:.4}");
    assert!(per_rich > 3.0 * per_sand && rich > rich0, "rich {per_rich} sand {per_sand}");
}

#[test]
fn the_plant_pass_with_five_thousand_plants() {
    let mut s = garden("grow-cost");
    fair_weather(&mut s);
    let sprout = s.world.defs.thing_id("garden:sprout").unwrap();
    let m = &s.world.map;
    let open: Vec<IVec> =
        (0..m.plane()).map(|i| m.pos(i)).filter(|&p| m.passable(p) && m.fixture_at(p).is_none()).take(5000).collect();
    assert_eq!(open.len(), 5000);
    for p in open {
        let e = s.world.spawn_fixture(sprout, p, false).unwrap();
        s.world.plant_seedling(e);
    }
    let plants = s.world.ecs.query::<&Growth>().iter().count();
    assert!(plants >= 5000, "{plants} plants");
    // Counted, not timed (DESIGN.md §8a): a pass that stopped staggering
    // works out every plant at once.
    let worked: Vec<usize> = (0..GROW_EVERY)
        .map(|_| {
            s.world.tick += PLANT_PASS;
            systems::grow(&mut s.world)
        })
        .collect();
    let most = worked.iter().max().unwrap();
    assert!(*most <= plants / GROW_EVERY as usize * 3 / 2, "a pass works out a share, not all {plants}: {worked:?}");
    assert!(worked.iter().sum::<usize>() >= plants, "every plant once in {GROW_EVERY} passes: {worked:?}");
}

/// A mod's harvest that gives `regrow_days` keeps to them on a plant that
/// grows, and old saves of such plants still load grown.
#[test]
fn regrow_days_still_work_for_mods_that_use_them() {
    let days = r##"
[[patch]]
target = "thing/core:berry_bush"
[[patch.edit]]
list = "harvest"
match = { designation = "harvest" }
set = { regrow_days = 1.5 }
"##;
    let dir = common::test_mods("grow-days", &["core"], &[("days", &[("defs/days.toml", days)])]);
    let mut s = Sim::new(&dir, 2).expect("mods load");
    let p = plot(&mut s, "grass", 7);
    let bush = sow(&mut s, "core:berry_bush", p);
    let harvest = s.world.defs.thing(s.world.defs.thing_id("berry_bush").unwrap()).harvest[0].key();
    assert!(s.world.harvest_ready(bush, harvest), "a harvest on days doesn't wait for the bush to grow");
}
