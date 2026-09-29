//! Standing orders (DESIGN.md §4d): priority rules on colony readings that
//! scripts publish, switching on past one mark and off past another, and
//! switched off by the colony when it doesn't want them.

mod common;

use rim_sim::data::Data;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{GameEvent, Pawn};
use rim_sim::{Command, Sim};
use std::path::PathBuf;

/// Core, and a mod with a "food is low" order on its own reading.
fn stores(name: &str, script: &str) -> PathBuf {
    let defs = r#"
[[priority_rule]]
id = "food_low"
label = "Food is low"
when = { reading = "stores:food_days", below = 5, until = 8 }
shift = { "core:harvest" = -1 }

[[priority_rule]]
id = "clutter"
when = { reading = "stores:loose", above = 30, until = 10 }
shift = { "core:haul" = -1 }
"#;
    let mut files = vec![("defs/orders.toml", defs)];
    if !script.is_empty() {
        files.push(("scripts/main.luau", script));
    }
    common::test_mods(name, &["core"], &[("stores", &files)])
}

fn harvest(s: &Sim) -> u8 {
    let pawn = s.world.colonists().next().unwrap();
    let p = s.world.ecs.get::<&Pawn>(pawn).unwrap();
    rim_sim::rules::effective(&s.world, &p, s.world.defs.lookup("work_type", "core:harvest").unwrap())
}

fn holds(s: &Sim, rule: &str) -> bool {
    s.world.standing.on.contains(rule)
}

#[test]
fn an_order_turns_on_under_its_mark_and_off_past_until() {
    let dir = stores("orders-band", "");
    let mut s = Sim::new(&dir, 1).unwrap();
    assert_eq!(harvest(&s), 3, "no reading, no order");
    for (food, on) in [(7.0, false), (5.0, false), (4.9, true), (6.0, true), (7.999, true), (8.0, false), (6.0, false)]
    {
        s.world.set_reading("stores:food_days", food);
        assert_eq!(holds(&s, "stores:food_low"), on, "at {food} days");
        assert_eq!(harvest(&s), if on { 2 } else { 3 }, "at {food} days");
    }
    // Above works the other way round.
    for (loose, on) in [(30.0, false), (31.0, true), (11.0, true), (10.0, false)] {
        s.world.set_reading("stores:loose", loose);
        assert_eq!(holds(&s, "stores:clutter"), on, "at {loose} loose");
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_reading_inside_its_band_costs_no_rule_evaluation() {
    let dir = stores("orders-cheap", "");
    let mut s = Sim::new(&dir, 1).unwrap();
    s.world.set_reading("stores:food_days", 4.0);
    let before = s.world.rules.evaluations;
    for food in [4.5, 6.0, 7.0, 3.0, 7.9] {
        s.world.set_reading("stores:food_days", food);
    }
    assert_eq!(s.world.rules.evaluations, before, "no mark crossed, nothing worked out again");
    s.world.set_reading("stores:food_days", 9.0);
    assert_eq!(s.world.rules.evaluations, before + 1, "crossing `until` works the rules out once");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_crossing_is_announced_once() {
    let dir = stores("orders-events", "");
    let mut s = Sim::new(&dir, 1).unwrap();
    s.world.events.clear();
    for food in [6.0, 4.0, 3.0, 2.0, 6.0, 9.0, 10.0] {
        s.world.set_reading("stores:food_days", food);
    }
    let seen: Vec<String> = s
        .world
        .events
        .iter()
        .filter_map(|e| match e {
            GameEvent::RuleStarted { rule } => Some(format!("started {rule}")),
            GameEvent::RuleStopped { rule } => Some(format!("stopped {rule}")),
            _ => None,
        })
        .collect();
    assert_eq!(seen, ["started stores:food_low", "stopped stores:food_low"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_switched_off_order_never_applies_and_stays_off_after_a_load() {
    let dir = stores("orders-off", "");
    let mut s = Sim::new(&dir, 1).unwrap();
    let rule = s.world.defs.lookup("priority_rule", "stores:food_low").unwrap();
    s.push(Command::SetRuleEnabled { rule, on: false });
    s.step();
    // The colonist's own level, whatever this seed's founder started at;
    // the order shifts it down one.
    let base = harvest(&s);
    s.world.set_reading("stores:food_days", 2.0);
    assert!(holds(&s, "stores:food_low"), "the reading still crossed the mark");
    assert_eq!(harvest(&s), base, "but the colony switched the order off");

    let mut back = Snapshot::capture(&s).restore(&dir, &|_| true).unwrap();
    assert_eq!(back.world.state_hash(), s.world.state_hash(), "readings, orders and switches are saved");
    assert_eq!(harvest(&back), base, "still off");
    back.push(Command::SetRuleEnabled { rule, on: true });
    back.step();
    assert_eq!(harvest(&back), base - 1, "back on, and the reading is still low");
    let _ = std::fs::remove_dir_all(dir);
}

/// An order from a mod that's gone holds nothing, and doesn't stop the load.
#[test]
fn an_order_from_a_removed_mod_is_forgotten() {
    let dir = stores("orders-removed", "");
    let mut s = Sim::new(&dir, 1).unwrap();
    s.world.set_reading("stores:food_days", 2.0);
    let back = Snapshot::capture(&s).restore(&dir, &|m| m != "stores").unwrap();
    assert!(back.world.standing.on.is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn scripts_publish_readings_deterministically() {
    let script = r#"
rim.every(100, function()
    -- Food runs down a day at a time, then a harvest fills it back up.
    local food = 10 - (rim.tick() // 100) % 12
    rim.set_reading("food_days", food)
    rim.set_data("stores:seen", rim.reading("stores:food_days"))
end)
rim.on("rule_started", function(e)
    if string.sub(e.rule, 1, 7) == "stores:" then
        rim.set_data("stores:started", e.rule .. " " .. e.label .. " " .. e.reading)
    end
end)
"#;
    let dir = stores("orders-script", script);
    let run = |save_at: Option<u64>| {
        let mut s = Sim::new(&dir, 3).unwrap();
        for t in 0..1500u64 {
            if Some(t) == save_at {
                s = Snapshot::capture(&s).restore(&dir, &|_| true).unwrap();
            }
            s.step();
        }
        s
    };
    let s = run(None);
    assert_eq!(s.world.state_hash(), run(None).world.state_hash(), "two runs agree");
    assert_eq!(s.world.state_hash(), run(Some(650)).world.state_hash(), "a load mid-order carries on the same");
    assert!(matches!(s.world.data.get("stores:seen"), Some(Data::Num(_) | Data::Int(_))), "rim.reading reads it back");
    let started = format!("{:?}", s.world.data.get("stores:started"));
    assert!(started.contains("stores:food_low Food is low stores:food_days"), "{started}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_mod_publishes_only_its_own_readings() {
    let script = r#"
rim.every(10, function()
    local ok = pcall(rim.set_reading, "core:food_days", 1)
    rim.set_data("stores:refused", not ok)
end)
"#;
    let dir = stores("orders-own", script);
    let mut s = Sim::new(&dir, 1).unwrap();
    for _ in 0..11 {
        s.step();
    }
    assert!(matches!(s.world.data.get("stores:refused"), Some(Data::Bool(true))));
    assert!(s.world.standing.readings.is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_reading_rule_says_what_it_needs() {
    for (when, want) in [
        (r#"{ reading = "food_days", below = 5 }"#, "name the reading with its mod"),
        (r#"{ reading = "x:food", below = 5, until = 3 }"#, "far side"),
        (r#"{ reading = "x:food", above = 5, until = 8 }"#, "far side"),
        (r#"{ reading = "x:food" }"#, "takes `below` or `above`"),
        (r#"{ reading = "x:food", need = "core:food", below = 0.5 }"#, "not both"),
        (r#"{ stance = "core:normal", until = 3 }"#, "`until` belongs to a `reading`"),
    ] {
        let defs = format!("[[priority_rule]]\nid = \"bad\"\nwhen = {when}\nshift = {{ \"core:haul\" = -1 }}\n");
        let dir = common::test_mods("orders-bad", &["core"], &[("x", &[("defs/r.toml", &defs)])]);
        let err = Sim::new(&dir, 1).err().unwrap_or_else(|| panic!("{when} should fail"));
        assert!(err.contains(want), "{when}: {err}");
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// A switched-off order keeps up with its reading without a word, and is
/// right the moment it's switched back on.
#[test]
fn a_switched_off_order_is_quiet_but_keeps_count() {
    let dir = stores("orders-quiet", "");
    let mut s = Sim::new(&dir, 1).unwrap();
    let rule = s.world.defs.lookup("priority_rule", "stores:food_low").unwrap();
    s.push(Command::SetRuleEnabled { rule, on: false });
    s.step();
    let base = harvest(&s);
    s.world.events.clear();
    let before = s.world.rules.evaluations;
    s.world.set_reading("stores:food_days", 2.0);
    assert!(s.world.events.is_empty(), "no news for an order that's off");
    assert_eq!(s.world.rules.evaluations, before, "nothing to work out");
    s.push(Command::SetRuleEnabled { rule, on: true });
    s.step();
    assert_eq!(harvest(&s), base - 1, "on again, and the reading was low all along");
    let _ = std::fs::remove_dir_all(dir);
}

/// A load on the very tick an order's hours begin: the game that kept
/// running announces it on its next step, and so does the loaded one.
#[test]
fn an_order_whose_hours_begin_at_the_load_is_announced() {
    let defs = r#"
[[priority_rule]]
id = "night_food"
when = { reading = "stores:food_days", below = 5, until = 8, hours = [18, 6] }
shift = { "core:harvest" = -1 }
"#;
    let script = r#"
rim.on("rule_started", function(e)
    if e.rule == "stores:night_food" then
        rim.set_data("started", (rim.get_data("started") or 0) + 1)
    end
end)
"#;
    let files = [("defs/orders.toml", defs), ("scripts/main.luau", script)];
    let dir = common::test_mods("orders-load-hour", &["core"], &[("stores", &files)]);
    let mut s = Sim::new(&dir, 1).unwrap();
    s.world.set_reading("stores:food_days", 2.0);
    while s.world.hour() as u32 != 18 {
        s.step();
    }
    let started = |s: &Sim| s.world.data.get("stores:started").cloned();
    assert_eq!(started(&s), None, "its hours begin on the next step");
    let mut back = Snapshot::capture(&s).restore(&dir, &|_| true).unwrap();
    s.step();
    back.step();
    assert!(started(&s).is_some(), "the live game announces it");
    assert_eq!(started(&back), started(&s), "and so does the loaded one");
    let _ = std::fs::remove_dir_all(dir);
}
