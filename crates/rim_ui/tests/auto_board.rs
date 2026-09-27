//! Auto on the board (DESIGN.md §4d): rings on planned cells, a plan for
//! one colonist, a settler joining, and the first day's hint.

mod common;

use common::*;
use rim_sim::{Sim, TICKS_PER_DAY};
use rim_ui::view::UiAction;
use rim_ui::{Input, Ui};

/// A colony of `n` on Auto, an hour in, with the Work screen open.
fn auto(n: usize) -> (Sim, Ui, rim_ui::view::ClientView) {
    let mut sim = sim_at(&mods());
    let human = sim.world.defs.creature_id("human").unwrap();
    let c = sim.world.colony_center().unwrap();
    for i in 1..n {
        sim.world.spawn_pawn(human, rim_sim::world::Faction::Player, c.offset(i as i32, 1), None);
    }
    for _ in 0..TICKS_PER_DAY / 24 + 1 {
        sim.step();
    }
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    ui.open_window("core:work");
    frame(&mut ui, &sim, &cv, Input { time: 0.5, ..Default::default() });
    (sim, ui, cv)
}

#[test]
fn planned_cells_wear_a_ring_and_a_click_pins_one() {
    let (sim, mut ui, mut cv) = auto(2);
    let cell = ui.grid_cell("core:work.grid", 1, 1).expect("the board, for two");
    assert!(cell.ring.is_some() && cell.dot.is_none(), "Auto chose it: {cell:?}");
    assert!(cell.tip.as_deref().unwrap_or("").contains("planned"), "the tip says so: {:?}", cell.tip);
    let g = ui.find("core:work.grid").unwrap();
    let at = (g[0] + 28.0, g[1] + 10.0);
    let actions = click(&mut ui, &sim, &mut cv, at);
    assert!(matches!(actions.as_slice(), [UiAction::SetPriority(..)]), "a click pins it: {actions:?}");
}

#[test]
fn one_colonist_is_a_plan_with_reasons() {
    let (sim, mut ui, mut cv) = auto(1);
    let tree = ui.snapshot();
    assert!(ui.find("core:work.grid").is_none(), "no grid for one colonist");
    assert!(tree.contains("Auto is running"), "{tree}");
    assert!(ui.find("core:work.plan.core:build").is_some(), "every work type on a shelf");
    assert!(
        tree.contains("nothing waiting") || tree.contains("fills") || tree.contains("covered"),
        "Auto's reasons: {tree}"
    );
    let show = ui.find("core:work.show_board").unwrap();
    click(&mut ui, &sim, &mut cv, centre(show));
    frame(&mut ui, &sim, &cv, Input { time: 5.0, ..Default::default() });
    assert!(ui.find("core:work.grid").is_some(), "the board on request");
}

#[test]
fn a_settler_joining_is_asked_for_a_role_with_auto_chosen() {
    let (mut sim, mut ui, mut cv) = auto(1);
    let human = sim.world.defs.creature_id("human").unwrap();
    let c = sim.world.colony_center().unwrap();
    let mira = sim.world.spawn_pawn(human, rim_sim::world::Faction::Player, c.offset(2, 2), Some("Mira".into()));
    sim.step();
    // Over the map, where a settler turns up; the Work sheet would cover it.
    ui.close_window("core:work");
    frame(&mut ui, &sim, &cv, Input { time: 5.0, ..Default::default() });
    assert!(ui.find("core:settler").is_some(), "the dialog: {}", ui.snapshot());
    assert!(ui.snapshot().contains("Mira joined the colony"));
    let builder = sim.world.work_roles.iter().position(|r| r.label == "Builder").unwrap() as u16;
    let b = ui.find(&format!("core:settler.role.{}", builder + 1)).expect("Builder offered");
    let actions = click(&mut ui, &sim, &mut cv, centre(b));
    assert_eq!(actions, vec![UiAction::AssignWorkRole(mira, builder)]);
    frame(&mut ui, &sim, &cv, Input { time: 6.0, ..Default::default() });
    assert!(ui.find("core:settler").is_none(), "answered, it goes");
}

#[test]
fn the_first_day_hint_shows_once() {
    let (sim, mut ui, mut cv) = auto(1);
    // On the map, not over the Work sheet.
    ui.close_window("core:work");
    frame(&mut ui, &sim, &cv, Input { time: 1.0, ..Default::default() });
    frame(&mut ui, &sim, &cv, Input { time: 1.5, ..Default::default() });
    assert!(ui.find("core:auto_hint").is_some(), "a castaway on Auto gets the hint");
    let ok = ui.find("core:auto_hint.ok").unwrap();
    click(&mut ui, &sim, &mut cv, centre(ok));
    frame(&mut ui, &sim, &cv, Input { time: 5.0, ..Default::default() });
    assert!(ui.find("core:auto_hint").is_none(), "once");
}
