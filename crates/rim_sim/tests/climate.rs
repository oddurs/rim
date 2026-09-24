//! Climate mechanisms: terms for outdoor values, named contributions, pins,
//! the calendar, script data and script events (DESIGN.md §4c).

use rim_sim::data::Data;
use rim_sim::{Sim, TICKS_PER_DAY};
use std::fs;

mod common;
use common::{mods, test_mods};

fn core_only(seed: u64) -> Sim {
    Sim::with_mods(&mods(), seed, &|m| m == "core").expect("core loads")
}

fn with_test_mods(name: &str, extra: &[(&str, &[(&str, &str)])]) -> std::path::PathBuf {
    test_mods(name, &["core"], extra)
}

fn field(s: &Sim, id: &str) -> usize {
    s.world.defs.lookup("field", id).unwrap() as usize
}

fn step_to(s: &mut Sim, tick: u64) {
    while s.world.tick < tick {
        s.step();
    }
}

/// The curve core's old climate script computed (smoothstep, 03:00 to 15:00).
fn old_curve(hour: f64) -> f64 {
    let mut d = (hour - 15.0).abs();
    if d > 12.0 {
        d = 24.0 - d;
    }
    let x = d / 12.0;
    10.0 + 9.0 * (1.0 - 2.0 * x * x * (3.0 - 2.0 * x))
}

#[test]
fn core_day_is_data_and_matches_the_old_script() {
    let mut s = core_only(3);
    let t = field(&s, "temperature");
    let l = field(&s, "light");
    let mut worst: f64 = 0.0;
    for hour in 0..72u64 {
        step_to(&mut s, hour * TICKS_PER_DAY / 24 + 1);
        let h = s.world.hour();
        worst = worst.max((s.world.fields.ambient(t) - old_curve(h)).abs());
    }
    assert!(worst < 1.5, "core's day is within {worst:.2}° of the old curve, want < 1.5°");
    // Light: full at noon, none at midnight.
    step_to(&mut s, 3 * TICKS_PER_DAY + TICKS_PER_DAY / 4);
    assert_eq!(s.world.fields.ambient(l), 100.0, "noon is full daylight");
    step_to(&mut s, 3 * TICKS_PER_DAY + TICKS_PER_DAY * 3 / 4);
    assert_eq!(s.world.fields.ambient(l), 0.0, "midnight is dark");
}

#[test]
fn pushes_ease_in_add_up_expire_and_explain_themselves() {
    let mut s = core_only(3);
    let t = field(&s, "temperature");
    let hour = TICKS_PER_DAY / 24;
    let now = s.world.tick;
    let base = |s: &Sim| s.world.fields.explain_ambient(&s.world.defs, t)[..2].iter().map(|p| p.1).sum::<f64>();
    s.world.fields.push_ambient(t, "cold_snap", -10.0, now, Some(4.0), 2.0);
    step_to(&mut s, now + hour);
    let part = s.world.fields.ambient(t) - base(&s);
    assert!((part + 5.0).abs() < 0.2, "halfway through easing in: {part:.2}");
    step_to(&mut s, now + 2 * hour + 20);
    s.world.fields.push_ambient(t, "fire_storm", 3.0, s.world.tick, None, 0.0);
    let at = s.world.tick + 21;
    step_to(&mut s, at);
    let parts = s.world.fields.explain_ambient(&s.world.defs, t);
    let labels: Vec<&str> = parts.iter().map(|p| p.0.as_str()).collect();
    assert_eq!(labels, ["day", "mean", "cold_snap", "fire_storm"]);
    let sum: f64 = parts.iter().map(|p| p.1).sum();
    assert!((sum - s.world.fields.ambient(t)).abs() < 0.01, "the breakdown adds up");
    assert!((parts[2].1 + 10.0).abs() < 0.01 && (parts[3].1 - 3.0).abs() < 0.01, "{parts:?}");
    // Expires after 4 hours, easing out over 2, then disappears.
    step_to(&mut s, now + 7 * hour);
    let parts = s.world.fields.explain_ambient(&s.world.defs, t);
    assert!(!parts.iter().any(|p| p.0 == "cold_snap"), "expired push is gone: {parts:?}");
    // A pin overrides everything until cleared.
    s.world.fields.set_ambient(t, Some(-30.0));
    let at = s.world.tick + 40;
    step_to(&mut s, at);
    assert_eq!(s.world.fields.ambient(t), -30.0);
    s.world.fields.set_ambient(t, None);
    let at = s.world.tick + 40;
    step_to(&mut s, at);
    assert!(s.world.fields.ambient(t) > -20.0, "unpinned");
}

#[test]
fn a_patch_changes_one_term_by_label() {
    let dir = with_test_mods(
        "patch",
        &[(
            "flat",
            &[(
                "defs/patch.toml",
                "[[patch]]\ntarget = \"field/temperature\"\nset = { ambient = { day = { scale = 0.0 }, mean = { of = [4.0] } } }\n",
            )],
        )],
    );
    let mut s = Sim::new(&dir, 1).expect("loads");
    let t = field(&s, "temperature");
    for h in [0, 6, 12, 18] {
        step_to(&mut s, h * TICKS_PER_DAY / 24 + 1);
        assert_eq!(s.world.fields.ambient(t), 4.0, "flat at hour {h}");
    }
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn bad_terms_and_cycles_fail_to_load_with_names() {
    let cyc = "[[field]]\nid = \"a\"\nlabel = \"a\"\nrange = [0.0, 1.0]\ncolor_low = \"#000000\"\ncolor_high = \"#ffffff\"\n[field.ambient.x]\nof = [{ ambient = \"b\" }]\n\
               [[field]]\nid = \"b\"\nlabel = \"b\"\nrange = [0.0, 1.0]\ncolor_low = \"#000000\"\ncolor_high = \"#ffffff\"\n[field.ambient.y]\nof = [{ ambient = \"a\" }]\n";
    let dir = with_test_mods("cycle", &[("loop", &[("defs/f.toml", cyc)])]);
    let err = Sim::new(&dir, 1).err().expect("a cycle must not load");
    assert!(err.contains("cycle") && err.contains("a -> b -> a"), "{err}");
    let _ = fs::remove_dir_all(dir);

    let bad = "[[patch]]\ntarget = \"field/temperature\"\nset = { ambient = { day = { of = [{ input = \"moon\" }] } } }\n";
    let dir = with_test_mods("badinput", &[("moon", &[("defs/p.toml", bad)])]);
    let err = Sim::new(&dir, 1).err().expect("an unknown input must not load");
    assert!(err.contains("field/temperature, term 'day'") && err.contains("moon"), "{err}");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn calendar_counts_days_and_seasons() {
    let mut s = core_only(3);
    let c = s.world.defs.calendar.clone();
    assert_eq!((c.year_days, c.seasons.len(), c.start_day), (60, 4, 8));
    assert_eq!((s.world.season(), s.world.day_of_season()), ("spring", 9));
    s.world.tick = 7 * TICKS_PER_DAY;
    assert_eq!((s.world.season(), s.world.day_of_season()), ("summer", 1));
    s.world.tick = 52 * TICKS_PER_DAY;
    assert_eq!((s.world.season(), s.world.day_of_season(), s.world.year()), ("spring", 1, 1));
    let clock = s.world.clock();
    assert_eq!(clock.year, 0, "a new year starts at 0");
}

#[test]
fn seasons_announce_themselves_once_each() {
    let script = r#"
        rim.on("season_changed", function(e)
            local seen = rim.get_data("t:seen") or {}
            table.insert(seen, e.season .. " " .. e.year)
            rim.set_data("t:seen", seen)
        end)
    "#;
    let dir = with_test_mods("seasons", &[("watch", &[("scripts/watch.luau", script)])]);
    let mut s = Sim::new(&dir, 5).expect("loads");
    // Jump to just before each season boundary over two years.
    for boundary_day in [7u64, 22, 37, 52, 67, 82, 97, 112] {
        s.world.tick = boundary_day * TICKS_PER_DAY - 2;
        for _ in 0..4 {
            s.step();
        }
    }
    let Some(Data::Table(seen)) = s.world.data.get("t:seen") else { panic!("no seasons seen") };
    let names: Vec<String> =
        seen.values().map(|v| if let Data::Str(x) = v { x.clone() } else { String::new() }).collect();
    assert_eq!(
        names,
        ["summer 1", "autumn 1", "winter 1", "spring 2", "summer 2", "autumn 2", "winter 2", "spring 3"]
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn script_data_and_events_cross_mods() {
    let sender = r#"
        rim.every(10, function()
            if rim.get_data("a:sent") then return end
            rim.set_data("a:sent", true)
            rim.emit("a:hello", { from = "a", n = 3, list = { 1, 2.5, "x" } })
        end)
    "#;
    let listener = r#"
        rim.on("a:hello", function(e)
            rim.set_data("b:got", { from = e.from, n = e.n, third = e.list[3] })
        end)
        rim.every(50, function()
            rim.set_data("b:bad", function() end)
        end)
    "#;
    let dir = with_test_mods("events", &[("a", &[("scripts/a.luau", sender)]), ("b", &[("scripts/b.luau", listener)])]);
    let mut s = Sim::new(&dir, 5).expect("loads");
    let h0 = s.world.state_hash();
    for _ in 0..60 {
        s.step();
    }
    let got = s.world.data.get("b:got").expect("the other mod heard the event");
    assert_eq!(got.get("from"), Some(&Data::Str("a".into())));
    assert_eq!(got.get("n").and_then(|v| v.num()), Some(3.0));
    assert_eq!(got.get("third"), Some(&Data::Str("x".into())));
    assert!(
        s.world.messages.iter().any(|m| m.text.contains("only plain data")),
        "storing a function is refused with a clear error"
    );
    // Data is part of the state.
    let before = s.world.state_hash();
    s.world.data.insert("z".into(), Data::Int(1));
    assert_ne!(before, s.world.state_hash());
    assert_ne!(h0, before);
    let _ = fs::remove_dir_all(dir);
}
