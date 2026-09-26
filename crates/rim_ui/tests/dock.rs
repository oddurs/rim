//! The command dock: verbs at the bottom, a tray that reads in decision
//! order, a pill while placing, and the screens up in the top bar.

mod common;

use common::*;
use rim_ui::view::UiAction;
use rim_ui::{Input, Key};

fn press(
    ui: &mut rim_ui::Ui,
    sim: &rim_sim::Sim,
    cv: &rim_ui::view::ClientView,
    t: &mut f64,
    key: &str,
) -> Vec<UiAction> {
    *t += 0.1;
    let out = frame(ui, sim, cv, Input { pressed: vec![key.into()], time: *t, ..Default::default() });
    *t += 0.1;
    frame(ui, sim, cv, Input { time: *t, ..Default::default() });
    out.actions
}

fn settle(ui: &mut rim_ui::Ui, sim: &rim_sim::Sim, cv: &rim_ui::view::ClientView, t: &mut f64) {
    for _ in 0..2 {
        *t += 0.1;
        frame(ui, sim, cv, Input { time: *t, ..Default::default() });
    }
}

#[test]
fn the_build_tray_reads_group_then_thing_then_card() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    let mut t = 0.0;
    settle(&mut ui, &sim, &cv, &mut t);
    assert!(ui.find("core:dock.tray").is_none(), "no tray until a verb is picked");
    press(&mut ui, &sim, &cv, &mut t, "b");
    let tray = ui.find("core:dock.tray").expect("B opens the Build tray");
    let rail = ui.find("core:dock.groups.structure").expect("a group rail");
    let tiles = ui.find("core:toolbar.buttons").expect("the things");
    let card = ui.find("core:dock.card").expect("the blueprint card");
    assert!(rail[0] < tiles[0] && tiles[0] < card[0], "left to right: group, thing, card");
    let dock = ui.find("core:dock").unwrap();
    assert!(tray[1] + tray[3] <= dock[1] + 0.5, "the tray rises from the dock: {tray:?} over {dock:?}");
    let snap = ui.snapshot();
    assert!(snap.contains("BLUEPRINT") && snap.contains("COST"), "the card has its captions:\n{snap}");
    // Another group, from the rail.
    let furniture = ui.find("core:dock.groups.furniture").unwrap();
    let mut cv2 = cv.clone();
    click(&mut ui, &sim, &mut cv2, centre(furniture));
    settle(&mut ui, &sim, &cv2, &mut t);
    let bed = cv.tools.iter().find(|t| t.key.ends_with(":bed")).unwrap().key.clone();
    assert!(ui.find(&format!("core:toolbar.{bed}")).is_some(), "furniture shows the bed");
}

#[test]
fn picking_folds_to_the_pill_and_escape_unfolds() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    let mut t = 0.0;
    settle(&mut ui, &sim, &cv, &mut t);
    press(&mut ui, &sim, &cv, &mut t, "b");
    let wall = cv.tools.iter().find(|t| t.key.ends_with(":wall")).unwrap().key.clone();
    let tile = ui.find(&format!("core:toolbar.{wall}")).expect("a wall tile");
    let actions = click(&mut ui, &sim, &mut cv, centre(tile));
    assert!(actions.contains(&UiAction::Tool(wall.clone())), "{actions:?}");
    // The client puts the tool in hand.
    for tool in &mut cv.tools {
        tool.active = tool.key == wall;
    }
    settle(&mut ui, &sim, &cv, &mut t);
    assert!(
        ui.find("core:dock.pill").is_some() && ui.find("core:dock.tray").is_none(),
        "placing: the pill, not the tray"
    );
    let actions = press(&mut ui, &sim, &cv, &mut t, "escape");
    assert!(actions.contains(&UiAction::Tool("select".into())), "Escape drops the tool: {actions:?}");
    for tool in &mut cv.tools {
        tool.active = tool.key == "select";
    }
    settle(&mut ui, &sim, &cv, &mut t);
    assert!(ui.find("core:dock.tray").is_some(), "and the tray is back");
    press(&mut ui, &sim, &cv, &mut t, "escape");
    assert!(ui.find("core:dock.tray").is_none(), "a second Escape closes it");
}

#[test]
fn number_keys_pick_in_an_open_tray_and_set_the_speed_otherwise() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    let mut t = 0.0;
    settle(&mut ui, &sim, &cv, &mut t);
    let actions = press(&mut ui, &sim, &cv, &mut t, "1");
    assert!(actions.contains(&UiAction::Speed(1)), "no tray: 1 is normal speed: {actions:?}");
    press(&mut ui, &sim, &cv, &mut t, "q");
    let actions = press(&mut ui, &sim, &cv, &mut t, "2");
    let second = cv.tools.iter().filter(|t| t.category == "orders").nth(1).unwrap().key.clone();
    assert!(actions.contains(&UiAction::Tool(second)), "with Orders open, 2 picks the second order: {actions:?}");
    assert!(!actions.iter().any(|a| matches!(a, UiAction::Speed(_))), "and doesn't change the speed");
}

#[test]
fn finding_by_name_crosses_groups() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    let mut t = 0.0;
    settle(&mut ui, &sim, &cv, &mut t);
    press(&mut ui, &sim, &cv, &mut t, "b");
    press(&mut ui, &sim, &cv, &mut t, "/");
    t += 0.1;
    frame(&mut ui, &sim, &cv, Input { keys: "bed".chars().map(Key::Char).collect(), time: t, ..Default::default() });
    settle(&mut ui, &sim, &cv, &mut t);
    let bed = cv.tools.iter().find(|t| t.key.ends_with(":bed")).unwrap().key.clone();
    let wall = cv.tools.iter().find(|t| t.key.ends_with(":wall")).unwrap().key.clone();
    assert!(ui.find(&format!("core:toolbar.{bed}")).is_some(), "the bed is found from Structure:\n{}", ui.snapshot());
    assert!(ui.find(&format!("core:toolbar.{wall}")).is_none(), "and the wall is filtered out");
}

#[test]
fn the_screens_are_tabs_in_the_top_bar() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    let mut t = 0.0;
    settle(&mut ui, &sim, &cv, &mut t);
    let top = ui.find("core:topbar").unwrap();
    let dock = ui.find("core:dock").unwrap();
    for id in ["core:screens.core:work", "core:screens.core:zones", "core:screens.core:news"] {
        let r = ui.find(id).unwrap_or_else(|| panic!("{id} is shown"));
        assert!(r[1] >= top[1] && r[1] + r[3] <= top[1] + top[3] + 0.5, "{id} is in the top bar: {r:?}");
        assert!(r[1] + r[3] < dock[1], "{id} is off the dock");
    }
}

/// Only the open tray is built: closed, the dock is a handful of nodes.
#[test]
fn a_closed_dock_builds_only_its_verbs() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    let mut t = 0.0;
    settle(&mut ui, &sim, &cv, &mut t);
    let closed = ui.info.nodes;
    press(&mut ui, &sim, &cv, &mut t, "b");
    let open = ui.info.nodes;
    assert!(open > closed + 20, "the tray is real when open ({closed} → {open})");
    press(&mut ui, &sim, &cv, &mut t, "b");
    assert_eq!(ui.info.nodes, closed, "and gone when closed");
}
