//! Urgent marks in the interface (DESIGN.md §4d): the map's menu offers
//! "Mark urgent" on a job, and "Clear urgent" once it has one.

mod common;

use common::*;
use rim_sim::Command;
use rim_ui::view::UiAction;
use rim_ui::Input;

/// The menu's row labels, top to bottom.
fn rows(ui: &rim_ui::Ui) -> Vec<String> {
    let snap = ui.snapshot();
    let mut out = Vec::new();
    let mut lines = snap.lines();
    while let Some(l) = lines.next() {
        if l.contains("#core:menu.row.") {
            if let Some(q) = lines.by_ref().find_map(|n| n.split('"').nth(1).map(str::to_string)) {
                out.push(q);
            }
        }
    }
    out
}

#[test]
fn the_map_menu_marks_a_job_urgent_and_clears_it() {
    let mut sim = sim_at(&mods());
    let defs = sim.world.defs.clone();
    let c = sim.world.colony_center().unwrap();
    let (wall, wood) = (defs.thing_id("wall").unwrap(), defs.thing_id("wood").unwrap());
    let spot = (2..12)
        .map(|d| c.offset(d, 0))
        .find(|&p| sim.world.map.passable(p) && sim.world.map.fixture_at(p).is_none())
        .unwrap();
    sim.push(Command::Build { thing: wall, stuff: Some(wood), a: spot, b: spot, facing: 0 });
    sim.step();
    let bp = sim.world.markable_at(spot).expect("the blueprint is a job");
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    let frame_at = |ui: &mut rim_ui::Ui, sim: &rim_sim::Sim, t: f64| {
        ui.frame(&sim.world, &cv, &Input { time: t, ..Default::default() })
    };
    frame_at(&mut ui, &sim, 0.1);
    let id = format!("{},{}", spot.x, spot.y);
    assert!(ui.context(&sim.world, &cv, "tile", &id, (400.0, 300.0)));
    frame_at(&mut ui, &sim, 0.2);
    let labels = rows(&ui);
    let i = labels.iter().position(|l| l == "Mark urgent").unwrap_or_else(|| panic!("a Mark urgent row: {labels:?}"));
    let row = ui.find(&format!("core:menu.row.{}", i + 1)).unwrap();
    let mut cv2 = cv.clone();
    let actions = click(&mut ui, &sim, &mut cv2, centre(row));
    assert!(actions.contains(&UiAction::MarkUrgent(bp, true)), "{actions:?}");

    sim.push(Command::MarkUrgent { target: bp, on: true });
    sim.step();
    frame_at(&mut ui, &sim, 1.0);
    assert!(ui.context(&sim.world, &cv, "tile", &id, (400.0, 300.0)));
    frame_at(&mut ui, &sim, 1.1);
    assert!(rows(&ui).contains(&"Clear urgent".to_string()), "{:?}", rows(&ui));
}

/// The HUD counts live marks.
#[test]
fn the_hud_counts_urgent_marks() {
    let mut sim = sim_at(&mods());
    let defs = sim.world.defs.clone();
    let c = sim.world.colony_center().unwrap();
    let (wall, wood) = (defs.thing_id("wall").unwrap(), defs.thing_id("wood").unwrap());
    let spot = (2..12)
        .map(|d| c.offset(d, 0))
        .find(|&p| sim.world.map.passable(p) && sim.world.map.fixture_at(p).is_none())
        .unwrap();
    sim.push(Command::Build { thing: wall, stuff: Some(wood), a: spot, b: spot, facing: 0 });
    sim.step();
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    ui.frame(&sim.world, &cv, &Input { time: 0.1, ..Default::default() });
    assert!(ui.find("core:urgent.count").is_none(), "no marks, no count");
    let bp = sim.world.markable_at(spot).unwrap();
    sim.push(Command::MarkUrgent { target: bp, on: true });
    sim.step();
    ui.frame(&sim.world, &cv, &Input { time: 0.5, ..Default::default() });
    assert!(ui.snapshot().contains("1 urgent"), "{}", ui.snapshot());
}

/// With a Later wall marked urgent, the why panel's pick names the mark,
/// and hovering the wall says it's urgent.
#[test]
fn the_why_panel_and_the_hover_say_urgent() {
    let mut sim = rim_sim::Sim::with_mods(&mods(), 2, &|m| m == "core").unwrap();
    hands(&mut sim);
    let pawn = sim.world.colonists().next().unwrap();
    let defs = sim.world.defs.clone();
    let work = |id: &str| defs.lookup("work_type", id).unwrap();
    sim.push(Command::SetPriority { pawn, work: work("core:harvest"), level: 2 });
    sim.push(Command::SetPriority { pawn, work: work("core:build"), level: 3 });
    let c = sim.world.pawn_pos(pawn).unwrap();
    let (wall, wood) = (defs.thing_id("wall").unwrap(), defs.thing_id("wood").unwrap());
    sim.world.place_item(wood, c.offset(1, 1), 20);
    let spot = (3..12)
        .map(|d| c.offset(d, 0))
        .find(|&p| sim.world.map.passable(p) && sim.world.map.fixture_at(p).is_none())
        .unwrap();
    sim.push(Command::Build { thing: wall, stuff: Some(wood), a: spot, b: spot, facing: 0 });
    let harvest = defs.lookup("designation", "harvest").unwrap();
    sim.push(Command::Designate { designation: harvest, a: c.offset(-25, -25), b: c.offset(25, 25) });
    sim.step();
    let bp = sim.world.markable_at(spot).expect("the blueprint is a job");
    sim.push(Command::MarkUrgent { target: bp, on: true });
    sim.step();

    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.selected = Some(pawn);
    frame(&mut ui, &sim, &cv, Input::default());
    let tab = ui.find("core:inspector.tabs.work").expect("a work tab");
    click(&mut ui, &sim, &mut cv, centre(tab));
    frame(&mut ui, &sim, &cv, Input { time: 5.0, ..Default::default() });
    let snap = ui.snapshot();
    assert!(snap.contains("(urgent: a level sooner)\""), "the pick names the mark: {snap}");

    cv.hover_cell = Some(spot);
    frame(&mut ui, &sim, &cv, Input { time: 10.0, ..Default::default() });
    let snap = ui.snapshot();
    assert!(snap.contains("\"Urgent · "), "the hover forecast says urgent: {snap}");
}
