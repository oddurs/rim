//! Compact mode and UI scale: a small screen keeps most of itself for the
//! map, and the UI scales with hit testing that matches what is drawn.

mod common;

use common::*;
use rim_ui::view::UiAction;
use rim_ui::Input;

fn settle(ui: &mut rim_ui::Ui, sim: &rim_sim::Sim, cv: &rim_ui::view::ClientView, t: &mut f64) {
    for _ in 0..3 {
        *t += 0.1;
        frame(ui, sim, cv, Input { time: *t, ..Default::default() });
    }
}

#[test]
fn a_small_screen_keeps_at_least_half_of_itself_for_the_map() {
    let mut sim = sim_at(&mods());
    let human = sim.world.defs.creature_id("human").unwrap();
    let c = sim.world.colony_center().unwrap();
    for i in 0..9 {
        let p = c.offset(i % 3 - 1, i / 3 + 1);
        if sim.world.map.passable(p) {
            sim.world.spawn_pawn(human, rim_sim::world::Faction::Player, p, None);
        }
    }
    for i in 0..4 {
        sim.world.message(format!("something happened {i}"), rim_sim::world::MsgKind::Info);
    }
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.screen = (1280.0, 720.0);
    cv.hover_cell = Some(c);
    let mut t = 0.0;
    settle(&mut ui, &sim, &cv, &mut t);
    let snap = ui.snapshot();
    assert!(!snap.contains("F12 devtools"), "compact: the key line goes:\n{snap}");
    let panels = [
        "core:topbar",
        "core:toolbar",
        "core:colonists",
        "core:inspector",
        "core:alerts",
        "core:messages",
        "core:hover",
    ];
    let covered: f32 = panels.iter().filter_map(|id| ui.find(id)).map(|r| r[2] * r[3]).sum();
    let share = covered / (1280.0 * 720.0);
    println!("panels cover {:.0}% of 1280×720", share * 100.0);
    assert!(share <= 0.5, "the map keeps at least half the screen; panels cover {:.0}%", share * 100.0);
}

/// A probe that shows what the views say about the screen.
const PROBE: &str = r#"
ui.define("probe:screen", function(view)
    local w, h = view.screen()
    return ui.text({ string.format("screen=%dx%d compact=%s scale=%.3f", w, h, tostring(view.compact()), view.ui_scale()), id = "probe:screen" })
end)
ui.mount("top", "probe:screen", { order = 90 })
"#;

#[test]
fn the_ui_scales_and_clicks_land_where_it_draws() {
    let dir = scratch_mods("scale", &[("probe", "", &[("ui/probe.luau", PROBE)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    let mut t = 0.0;
    settle(&mut ui, &sim, &cv, &mut t);
    assert!(ui.snapshot().contains("screen=1600x960 compact=false scale=1.000"), "{}", ui.snapshot());
    let base = ui.find("core:dock.orders").unwrap();
    for s in [0.75f32, 1.0, 1.5, 2.0] {
        ui.set_user_scale(s);
        // The client passes the total scale on, as it does each frame.
        cv.scale = s;
        settle(&mut ui, &sim, &cv, &mut t);
        let orders = ui.find("core:dock.orders").unwrap();
        let ratio = orders[3] / base[3];
        assert!((ratio - s).abs() < 0.15, "at {s} the button is {ratio:.2}× as tall");
        // A click on its centre opens its palette: hits match drawing.
        assert!(ui.find("core:toolbar.buttons").is_none());
        click(&mut ui, &sim, &mut cv, centre(orders));
        settle(&mut ui, &sim, &cv, &mut t);
        let chop = ui.find("core:toolbar.designate:core:chop").unwrap_or_else(|| panic!("at {s}: {}", ui.snapshot()));
        let actions = click(&mut ui, &sim, &mut cv, centre(chop));
        assert_eq!(actions, vec![UiAction::Tool("designate:core:chop".into())], "at {s}");
        // Close it again for the next scale.
        let orders = ui.find("core:dock.orders").unwrap();
        click(&mut ui, &sim, &mut cv, centre(orders));
        settle(&mut ui, &sim, &cv, &mut t);
    }
    // Logical pixels: at 1.5 a 1600-pixel screen is 1067 wide, and compact.
    ui.set_user_scale(1.5);
    cv.scale = 1.5;
    settle(&mut ui, &sim, &cv, &mut t);
    assert!(ui.snapshot().contains("screen=1066x640 compact=true scale=1.500"), "{}", ui.snapshot());
    // Ctrl+= steps it up through the binding.
    t += 0.1;
    let out = frame(&mut ui, &sim, &cv, Input { pressed: vec!["ctrl+=".into()], time: t, ..Default::default() });
    assert!(out.actions.contains(&UiAction::UiScale(1.625)), "{:?}", out.actions);
    let _ = std::fs::remove_dir_all(&dir);
}
