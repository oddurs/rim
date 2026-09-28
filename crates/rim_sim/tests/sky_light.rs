//! Core's sun and moon light the sim from where they are (DESIGN.md §6e):
//! daylight follows the sun through the seasons, and a full moon gives a
//! little light at night, enough for a plant to grow by.

mod common;

use rim_sim::field::AMBIENT_INTERVAL;
use rim_sim::systems::{self, PLANT_PASS};
use rim_sim::world::Growth;
use rim_sim::{Sim, TICKS_PER_DAY};

const HOUR: u64 = TICKS_PER_DAY / 24;

fn core() -> Sim {
    Sim::build(&common::mods(), 1, &|id| id == "core", 32).unwrap()
}

fn field(s: &Sim, id: &str) -> usize {
    s.world.defs.lookup("field", id).unwrap() as usize
}

fn pin(s: &mut Sim, id: &str, v: f64) {
    let f = field(s, id);
    s.world.fields.set_ambient(f, Some(v));
}

/// The outdoor light at `tick`, under a clear sky.
fn light_at(s: &mut Sim, tick: u64) -> f64 {
    s.world.tick = tick;
    let (defs, clock) = (s.world.defs.clone(), s.world.clock());
    s.world.fields.update_ambient(&defs, clock);
    s.world.fields.ambient(field(s, "light"))
}

/// Core's moon is full at midnight after the first night (tick 0 is 06:00),
/// and every 15 days after; new halfway between.
fn midnight(day: u64) -> u64 {
    day * TICKS_PER_DAY + 18 * HOUR
}
const FULL: u64 = 15;
const NEW: u64 = 22;

#[test]
fn a_full_moon_lights_the_night_and_a_new_one_doesnt() {
    let mut s = core();
    pin(&mut s, "cloud", 0.0);
    let full = light_at(&mut s, midnight(FULL));
    let new = light_at(&mut s, midnight(NEW));
    println!("midnight light: full moon {full}, new moon {new}");
    assert!(full > 1.0, "a full moon lights the night: {full}");
    assert_eq!(new, 0.0, "a new moon gives none");
    // Cloud dims it as it dims the sun.
    pin(&mut s, "cloud", 100.0);
    let clouded = light_at(&mut s, midnight(FULL));
    assert!(clouded > 0.0 && clouded < full * 0.6, "{clouded} under cloud, {full} clear");
}

/// Hours of the day at `day` with light enough for a plant to grow fully
/// (30), and the light at noon.
fn day_of(s: &mut Sim, day: u64) -> (f64, f64) {
    let start = day * TICKS_PER_DAY;
    let steps = TICKS_PER_DAY / AMBIENT_INTERVAL;
    let lit = (0..steps).filter(|i| light_at(s, start + i * AMBIENT_INTERVAL) >= 30.0).count();
    (lit as f64 * AMBIENT_INTERVAL as f64 / HOUR as f64, light_at(s, start + 6 * HOUR))
}

#[test]
fn daylight_follows_the_seasons() {
    let mut s = core();
    pin(&mut s, "cloud", 0.0);
    let cal = s.world.defs.calendar.clone();
    // Core's year is 60 days from the game's start day; midsummer 0.375 of it.
    let day = |frac: f64| ((frac * cal.year_days as f64) as u64 + cal.year_days as u64 - cal.start_day as u64) % 60;
    let (summer, summer_noon) = day_of(&mut s, day(0.375));
    let (winter, winter_noon) = day_of(&mut s, day(0.875));
    let (spring, _) = day_of(&mut s, day(0.125));
    println!("lit hours: midsummer {summer:.1} (noon {summer_noon}), equinox {spring:.1}, midwinter {winter:.1} (noon {winter_noon})");
    assert!(summer > spring + 2.0 && spring > winter + 2.0, "{summer} {spring} {winter}");
    assert!(summer_noon > winter_noon, "a high sun is brighter: {summer_noon} {winter_noon}");
}

#[test]
fn a_plant_grows_on_a_full_moon_night_and_not_a_new_one() {
    let mut s = core();
    pin(&mut s, "cloud", 0.0);
    pin(&mut s, "temperature", 20.0);
    let home = s.world.colony_center().unwrap();
    let grass = s.world.defs.lookup("terrain", "grass").unwrap();
    let def = s.world.defs.thing_id("berry_bush").unwrap();
    let m = &s.world.map;
    let cell = (0..m.plane())
        .map(|i| m.pos(i))
        .find(|&p| p.chebyshev(home) > 8 && m.terrain[m.idx(p)] == grass && m.fixture_at(p).is_none())
        .expect("open grass");
    let bush = s.world.spawn_fixture(def, cell, false).expect("planted");
    s.world.plant_seedling(bush);

    // From 22:00 to 02:00, when the sun is well down, the moon's light alone.
    let night = |s: &mut Sim, day: u64| {
        let before = s.world.ecs.get::<&Growth>(bush).unwrap().progress;
        light_at(s, midnight(day) - 2 * HOUR);
        for _ in 0..4 * HOUR {
            s.world.tick += 1;
            if s.world.tick.is_multiple_of(AMBIENT_INTERVAL) {
                let (defs, clock) = (s.world.defs.clone(), s.world.clock());
                s.world.fields.update_ambient(&defs, clock);
            }
            if s.world.tick.is_multiple_of(PLANT_PASS) {
                systems::grow(&mut s.world);
            }
        }
        s.world.ecs.get::<&Growth>(bush).unwrap().progress - before
    };
    let full = night(&mut s, FULL);
    let new = night(&mut s, NEW);
    println!("growth in four night hours: full moon {full}, new moon {new}");
    assert!(full > 0, "it grows by a full moon");
    assert_eq!(new, 0, "and not in the dark");
}
