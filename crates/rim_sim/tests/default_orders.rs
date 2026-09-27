//! Core's standing orders (DESIGN.md §4d): the readings core publishes
//! hourly, and the three orders on them.

mod common;

use rim_sim::defs::Category;
use rim_sim::world::{Pawn, Thing};
use rim_sim::{Command, Sim, TICKS_PER_DAY};

const HOUR: u64 = TICKS_PER_DAY / 24;

fn core() -> Sim {
    Sim::with_mods(&common::mods(), 1, &|m| m == "core").unwrap()
}

fn reading(s: &Sim, id: &str) -> f64 {
    s.world.standing.reading(id).unwrap_or_else(|| panic!("{id} not published"))
}

fn holds(s: &Sim, rule: &str) -> bool {
    let i = s.world.defs.lookup("priority_rule", rule).unwrap();
    s.world.rules.on.contains(&i)
}

fn level(s: &Sim, work: &str) -> u8 {
    let pawn = s.world.colonists().next().unwrap();
    let p = s.world.ecs.get::<&Pawn>(pawn).unwrap();
    rim_sim::rules::effective(&s.world, &p, s.world.defs.lookup("work_type", work).unwrap())
}

/// Each reading matches a count made the long way, over every stack.
#[test]
fn readings_match_a_hand_count() {
    let mut s = core();
    let c = s.world.colony_center().unwrap();
    let (berries, wood) = (s.world.defs.thing_id("berries").unwrap(), s.world.defs.thing_id("wood").unwrap());
    s.world.place_item(berries, c.offset(2, 2), 40);
    s.world.place_item(wood, c.offset(-2, 2), 25);
    for _ in 0..HOUR + 1 {
        s.step();
    }
    let defs = s.world.defs.clone();
    let (mut fed, mut loose, mut logs) = (0.0, 0u32, 0u32);
    for (_, t) in s.world.ecs.query::<(hecs::Entity, &Thing)>().iter() {
        let d = defs.thing(t.def);
        if d.category != Category::Item {
            continue;
        }
        fed += t.count as f64 * d.food.as_ref().map_or(0.0, |f| f.nutrition);
        loose += t.count;
        logs += if t.def == wood { t.count } else { 0 };
    }
    let colonists = s.world.colonists().count() as f64;
    let expect = fed / (colonists * 0.5);
    assert!((reading(&s, "core:food_days") - expect).abs() < 0.01, "{} vs {expect}", reading(&s, "core:food_days"));
    assert_eq!(reading(&s, "core:loose_items"), loose as f64, "no stockpile: everything is loose");
    assert_eq!(reading(&s, "core:wood"), logs as f64);
}

#[test]
fn each_order_starts_and_stops_at_its_marks() {
    let mut s = core();
    s.world.set_reading("core:food_days", 4.0);
    assert!(holds(&s, "core:food_low"));
    assert_eq!((level(&s, "core:harvest"), level(&s, "core:hunt")), (2, 2), "foraging a level sooner");
    s.world.set_reading("core:food_days", 7.0);
    assert!(holds(&s, "core:food_low"), "still under 8");
    s.world.set_reading("core:food_days", 8.0);
    assert!(!holds(&s, "core:food_low"));

    s.world.set_reading("core:loose_items", 31.0);
    assert!(holds(&s, "core:loose_items"));
    assert_eq!(level(&s, "core:haul"), 2);
    s.world.set_reading("core:loose_items", 10.0);
    assert!(!holds(&s, "core:loose_items"));
}

/// Wood before winter holds only in autumn, however low the wood.
#[test]
fn wood_before_winter_waits_for_autumn() {
    let mut s = core();
    s.world.set_reading("core:wood", 10.0);
    assert_eq!(s.world.season(), "spring");
    assert!(!holds(&s, "core:wood_for_winter"), "low wood in spring is no order");
    let autumn = s.world.defs.calendar.seasons.iter().position(|x| x == "autumn").unwrap() as u32;
    while s.world.season_index() != autumn {
        s.world.tick += TICKS_PER_DAY;
    }
    s.world.update_rules();
    assert!(holds(&s, "core:wood_for_winter"));
    assert_eq!(level(&s, "core:chop"), 2);
}

/// A player who doesn't want an order switches it off.
#[test]
fn an_order_switches_off() {
    let mut s = core();
    let rule = s.world.defs.lookup("priority_rule", "core:food_low").unwrap();
    s.push(Command::SetRuleEnabled { rule, on: false });
    s.step();
    s.world.set_reading("core:food_days", 1.0);
    assert_eq!(level(&s, "core:harvest"), 3);
}

/// Starting and stopping make the news.
#[test]
fn an_order_makes_the_news() {
    let mut s = core();
    s.world.messages.clear();
    s.world.set_reading("core:food_days", 3.4);
    s.step();
    s.world.set_reading("core:food_days", 9.0);
    s.step();
    let said: Vec<&str> = s.world.messages.iter().map(|m| m.text.as_str()).collect();
    assert!(said.contains(&"Standing order on: Food is low (3.4)."), "{said:?}");
    assert!(said.contains(&"Standing order off: Food is low (9)."), "{said:?}");
}
