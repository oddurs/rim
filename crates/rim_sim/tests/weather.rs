//! The weather plugin (mods/weather): the forecast is the future, weights
//! decide frequencies, channels ease, forcing works, and core stands alone.

mod common;

use common::{mods, test_mods};
use rim_sim::data::{Data, Key};
use rim_sim::{Sim, TICKS_PER_DAY};
use std::fs;

/// (id, start, ends) for each queued entry, current first.
fn queue(s: &Sim) -> Vec<(String, i64, i64)> {
    let Some(Data::Table(q)) = s.world.data.get("weather:forecast") else { return Vec::new() };
    q.values()
        .map(|e| {
            let id = match e.get("id") {
                Some(Data::Str(x)) => x.clone(),
                _ => String::new(),
            };
            let n = |k| e.get(k).and_then(|v| v.num()).unwrap_or(-1.0) as i64;
            (id, n("start"), n("ends"))
        })
        .collect()
}

#[test]
fn the_forecast_is_what_happens() {
    // A year in release (CI); a few days in debug.
    let days = if cfg!(debug_assertions) { 4 } else { 60 };
    for seed in [1, 2, 3] {
        let mut s = Sim::new(&mods(), seed).expect("mods load");
        let mut before: Vec<(String, i64, i64)> = Vec::new();
        let mut changes = 0;
        let mut forced = 0;
        for _ in 0..TICKS_PER_DAY * days {
            s.step();
            let now = queue(&s);
            if before.is_empty() {
                before = now;
                continue;
            }
            if now[0] != before[0] {
                changes += 1;
                let ids = |q: &[(String, i64, i64)]| q.iter().map(|e| e.0.clone()).collect::<Vec<_>>();
                if now[0].1 == s.world.tick as i64 - 1 && ids(&now[1..3]) == ids(&before[1..3]) {
                    // Forced by an incident: the current weather was replaced
                    // and what was forecast after it still comes, later.
                    forced += 1;
                } else {
                    // Everything that was forecast moved up one place, unchanged.
                    assert_eq!(now[..3], before[1..], "seed {seed}: the forecast was wrong at tick {}", s.world.tick);
                    assert!(now[0].1 <= s.world.tick as i64, "it started on time");
                }
            }
            before = now;
        }
        assert!(changes >= days, "seed {seed}: the weather changed {changes} times in {days} days");
        // "Under a fifth forced" is a claim about a year, not about the
        // five changes a debug run sees: one cold snap in four days is not
        // a broken forecast. Hold the ratio only once there is a sample.
        assert!(changes < 20 || forced * 5 < changes, "seed {seed}: {forced} of {changes} changes were forced");
    }
}

#[test]
fn frequencies_follow_the_weights() {
    // In mid-autumn after overcast weather, draw many picks and compare.
    let script = r#"
        local weather = require("@weather/scripts/weather")
        local done = false
        rim.every(1, function()
            if done then return end
            done = true
            local start = rim.tick() + 26 * rim.ticks_per_day
            local counts = {}
            for _ = 1, 20000 do
                local e = weather.pick("cloudy", start)
                counts[e.id] = (counts[e.id] or 0) + 1
            end
            rim.set_data("t:counts", counts)
            rim.set_data("t:weights", weather.weights("cloudy", start))
        end)
    "#;
    let dir = test_mods("weights", &["core", "weather"], &[("probe", &[("scripts/probe.luau", script)])]);
    let mut s = Sim::new(&dir, 9).expect("loads");
    for _ in 0..3 {
        s.step();
    }
    let table = |k: &str| -> Vec<(String, f64)> {
        let Some(Data::Table(t)) = s.world.data.get(k) else { panic!("no {k}") };
        t.iter().map(|(k, v)| (if let Key::Str(k) = k { k.clone() } else { String::new() }, v.num().unwrap())).collect()
    };
    let (counts, weights) = (table("t:counts"), table("t:weights"));
    let total_w: f64 = weights.iter().map(|w| w.1).sum();
    for (id, w) in &weights {
        let got = counts.iter().find(|c| &c.0 == id).map_or(0.0, |c| c.1) / 20000.0;
        let want = w / total_w;
        assert!((got - want).abs() < 0.05 * want.max(0.1), "{id}: {got:.3} of picks, weight share {want:.3}");
    }
    assert!(weights.iter().any(|w| w.0 == "weather:rain"), "rain is possible in autumn: {weights:?}");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn channels_ease_between_weathers() {
    let mut s = Sim::new(&mods(), 4).expect("mods load");
    let ids: Vec<usize> = ["cloud", "precipitation", "wind", "fog"]
        .iter()
        .map(|f| s.world.defs.lookup("field", f).unwrap() as usize)
        .collect();
    let mut last: Vec<f64> = ids.iter().map(|&f| s.world.fields.ambient(f)).collect();
    let mut worst: f64 = 0.0;
    for _ in 0..TICKS_PER_DAY * 3 / 20 {
        for _ in 0..20 {
            s.step();
        }
        for (i, &f) in ids.iter().enumerate() {
            let v = s.world.fields.ambient(f);
            // Normalise by each field's range: a jump is a big share of it.
            let r = &s.world.defs.fields[f].range;
            worst = worst.max((v - last[i]).abs() / (r[1] - r[0]));
            last[i] = v;
        }
    }
    // The fastest blend is an hour (833 ticks): at most ~2.4% of a range per 20 ticks.
    assert!(worst < 0.05, "a channel jumped {:.1}% of its range in 20 ticks", worst * 100.0);
}

#[test]
fn forcing_the_weather_and_hearing_about_it() {
    let script = r#"
        local weather = require("@weather/scripts/weather")
        rim.on("weather:changed", function(e)
            local log = rim.get_data("t:log") or {}
            table.insert(log, e.to)
            rim.set_data("t:log", log)
        end)
        rim.every(10, function()
            if rim.tick() >= 1000 and not rim.get_data("t:forced") then
                rim.set_data("t:forced", true)
                weather.force("storm", 3)
            end
        end)
    "#;
    let dir = test_mods("force", &["core", "weather"], &[("forcer", &[("scripts/forcer.luau", script)])]);
    let mut s = Sim::new(&dir, 2).expect("loads");
    for _ in 0..1100 {
        s.step();
    }
    let q = queue(&s);
    assert_eq!(q[0].0, "weather:storm", "the storm is the current weather: {q:?}");
    assert_eq!(q[0].2 - q[0].1, (3 * TICKS_PER_DAY / 24) as i64, "for three hours");
    assert!(q.windows(2).all(|w| w[0].2 == w[1].1), "the rest of the forecast follows on: {q:?}");
    let Some(Data::Table(log)) = s.world.data.get("t:log") else { panic!("no weather:changed events") };
    assert_eq!(log.values().last(), Some(&Data::Str("weather:storm".into())), "weather:changed fired for the storm");
    let wind = s.world.defs.lookup("field", "wind").unwrap() as usize;
    for _ in 0..TICKS_PER_DAY / 24 * 2 {
        s.step();
    }
    assert!(s.world.fields.ambient(wind) > 10.0, "storm winds: {}", s.world.fields.ambient(wind));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn core_alone_has_no_weather() {
    let mut s = Sim::with_mods(&mods(), 5, &|m| m == "core").expect("core loads");
    for _ in 0..TICKS_PER_DAY {
        s.step();
    }
    for f in ["cloud", "precipitation", "wind", "fog"] {
        let i = s.world.defs.lookup("field", f).unwrap() as usize;
        assert_eq!(s.world.fields.ambient(i), 0.0, "{f} is still with core alone");
        assert!(s.world.fields.atmos[i].pushes.is_empty());
    }
    assert!(s.world.data.is_empty(), "no plugin state");
}

/// A whole year, twice: weather, seasons and everything they touch must
/// replay exactly. Release only (CI runs tests in release).
#[test]
#[cfg_attr(debug_assertions, ignore = "a year of ticks; run with --release")]
fn a_year_of_weather_is_deterministic() {
    let run = || {
        let mut s = Sim::new(&mods(), 11).expect("mods load");
        for _ in 0..60 * TICKS_PER_DAY {
            s.step();
        }
        (s.world.state_hash(), s.world.year(), queue(&s))
    };
    let (a, year, q) = run();
    assert_eq!(year, 1, "a full year went by");
    assert!(!q.is_empty());
    assert_eq!(a, run().0);
}

#[test]
fn weather_incidents_show_in_the_breakdown_and_forecast() {
    let script = r#"
        local incidents = require("@weather/scripts/incidents")
        local step = 0
        rim.every(10, function()
            step += 1
            if step == 3 then
                incidents.fire("cold_snap")
                incidents.fire("heat_wave")
            elseif step == 4 then
                incidents.fire("storm")
            end
        end)
    "#;
    let dir = test_mods("incidents", &["core", "weather"], &[("trigger", &[("scripts/t.luau", script)])]);
    let mut s = Sim::new(&dir, 6).expect("loads");
    for _ in 0..60 {
        s.step();
    }
    let t = s.world.defs.lookup("field", "temperature").unwrap() as usize;
    let labels: Vec<String> = s.world.fields.explain_ambient(&s.world.defs, t).into_iter().map(|p| p.0).collect();
    assert!(labels.contains(&"cold_snap".to_string()), "{labels:?}");
    assert!(labels.contains(&"heat_wave".to_string()), "{labels:?}");
    assert_eq!(queue(&s)[0].0, "weather:storm", "the storm is in the forecast as the current weather");
    let texts: Vec<&str> = s.world.messages.iter().map(|m| m.text.as_str()).collect();
    assert!(texts.iter().any(|m| m.contains("cold snap")), "{texts:?}");
    assert!(texts.iter().any(|m| m.contains("storm")), "{texts:?}");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn the_weather_can_be_forced_by_a_command() {
    // What the devtools panel sends: a command, delivered to the weather
    // plugin's sim script as the event weather:force.
    let mut s = Sim::new(&mods(), 3).expect("mods load");
    for _ in 0..40 {
        s.step();
    }
    let mut data = std::collections::BTreeMap::new();
    data.insert(Key::Str("id".into()), Data::Str("fog".into()));
    data.insert(Key::Str("hours".into()), Data::Int(5));
    s.push(rim_sim::Command::ModEvent { name: "weather:force".into(), data: Some(Data::Table(data)) });
    s.step();
    s.step();
    let q = queue(&s);
    assert_eq!(q[0].0, "weather:fog", "{q:?}");
    assert!((q[0].2 - q[0].1 - (5 * TICKS_PER_DAY / 24) as i64).abs() <= 1, "five hours: {q:?}");
    let Some(Data::Table(types)) = s.world.data.get("weather:types") else { panic!("types are published") };
    assert!(types.values().any(|v| v == &Data::Str("weather:storm".into())));
}
