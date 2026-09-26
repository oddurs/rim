//! Now: the alert registry and the news, down the right edge.

mod common;

use common::*;
use rim_sim::world::MsgKind;
use rim_ui::view::UiAction;
use rim_ui::Input;

/// A mod that registers an alert, counts how often its check runs, and
/// registers one that errors.
const ALERT_MOD: &str = r#"
local alerts = require("@core/ui/alerts")
local runs = 0
alerts.add({
    id = "probe:always",
    severity = "bad",
    check = function(view)
        runs += 1
        return "probe alert " .. runs
    end,
})
alerts.add({ id = "probe:broken", severity = "info", check = function(view) error("kaboom") end })
ui.define("probe:runs", function(view) return ui.text({ "runs=" .. runs, id = "probe:runs" }) end)
ui.mount("top", "probe:runs", { order = 90 })
"#;

#[test]
fn a_mods_alert_sits_beside_cores_and_checks_run_four_times_a_second() {
    let dir = scratch_mods("now-alerts", &[("probe", "", &[("ui/alerts.luau", ALERT_MOD)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    // Core's own: with no stockpile, it says so.
    frame(&mut ui, &sim, &cv, Default::default());
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    let snap = ui.snapshot();
    assert!(ui.find("core:alerts.probe:always").is_some(), "the mod's alert is listed:\n{snap}");
    assert!(ui.find("core:alerts.core:no_stockpile").is_some(), "core's is listed beside it:\n{snap}");
    // Most severe first: the mod's bad alert above core's info.
    let probe = ui.find("core:alerts.probe:always").unwrap();
    let core = ui.find("core:alerts.core:no_stockpile").unwrap();
    assert!(probe[1] < core[1], "bad before info: {probe:?} {core:?}");
    // A check that errors shows, named, rather than vanishing.
    assert!(snap.contains("alert probe:broken") && snap.contains("kaboom"), "the broken check is named:\n{snap}");

    // A second of frames, each forced to rebuild by a hover change: the
    // checks still run at most four or five times.
    let c = sim.world.colony_center().unwrap();
    for i in 0..60 {
        cv.time = 1.0 + i as f64 / 60.0;
        cv.hover_cell = Some(c.offset(i % 7, 0));
        frame(&mut ui, &sim, &cv, Input { time: cv.time, ..Default::default() });
    }
    let snap = ui.snapshot();
    let runs: u32 = snap
        .split("runs=")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("no run count:\n{snap}"));
    assert!((4..=6).contains(&runs), "checks ran {runs} times in a second of rebuilds");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_alert_about_a_colonist_selects_them() {
    let sim = sim_at(&mods());
    let e = sim.world.colonists().next().unwrap();
    sim.world.ecs.get::<&mut rim_sim::world::Pawn>(e).unwrap().hp = 1;
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.selected = None;
    frame(&mut ui, &sim, &cv, Default::default());
    let hurt = ui.find("core:alerts.core:hurt").unwrap_or_else(|| panic!("hurt alert:\n{}", ui.snapshot()));
    let actions = click(&mut ui, &sim, &mut cv, centre(hurt));
    assert!(actions.contains(&UiAction::Select(Some(e))), "{actions:?}");
    assert!(actions.contains(&UiAction::Focus(e)), "{actions:?}");
}

/// Five hundred messages at once: the column shows the newest few, the
/// window has every one, and it builds only the lines in view.
#[test]
fn nothing_is_lost_when_many_messages_arrive() {
    let mut sim = sim_at(&mods());
    let before = sim.world.messages.len();
    for i in 0..500 {
        sim.world.message(format!("event {i}"), MsgKind::Info);
    }
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let snap = ui.snapshot();
    assert!(snap.contains("event 499") && !snap.contains("event 400"), "the newest few in the column:\n{snap}");
    assert!(snap.contains(&format!("All news · {}", before + 500)), "and a count of them all:\n{snap}");
    ui.open_window("core:news");
    for i in 1..3 {
        frame(&mut ui, &sim, &cv, Input { time: i as f64, ..Default::default() });
    }
    let snap = ui.snapshot();
    assert!(snap.contains("event 499"), "the window starts at the newest:\n{snap}");
    let lines = snap.matches("event ").count();
    assert!(lines < 100, "the window builds only the lines in view ({lines})");
}
