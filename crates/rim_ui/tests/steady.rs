//! The HUD holds still: things that come and go (a tray, the placing pill,
//! the undo chip) never move a docked panel, and a slotted panel's header
//! stays put while its content changes (DESIGN.md §11).

mod common;

use common::*;
use rim_sim::world::Thing;
use rim_ui::Input;

struct Run {
    ui: rim_ui::Ui,
    cv: rim_ui::view::ClientView,
    t: f64,
}

impl Run {
    fn frame(&mut self, sim: &rim_sim::Sim, input: Input) -> rim_ui::Output {
        self.t += 0.1;
        let out = frame(&mut self.ui, sim, &self.cv, Input { time: self.t, ..input });
        self.t += 0.1;
        frame(&mut self.ui, sim, &self.cv, Input { time: self.t, ..Default::default() });
        out
    }

    fn press(&mut self, sim: &rim_sim::Sim, key: &str) {
        self.frame(sim, Input { pressed: vec![key.into()], ..Default::default() });
    }

    fn click(&mut self, sim: &rim_sim::Sim, id: &str) {
        let r = self.ui.find(id).unwrap_or_else(|| panic!("{id} is on screen"));
        click(&mut self.ui, sim, &mut self.cv, centre(r));
        self.frame(sim, Input::default());
    }

    fn find(&self, id: &str) -> [f32; 4] {
        self.ui.find(id).unwrap_or_else(|| panic!("{id} is on screen"))
    }
}

/// What the client does with a picked tool: puts it in hand.
fn hand(cv: &mut rim_ui::view::ClientView, key: &str) {
    for tool in &mut cv.tools {
        tool.active = tool.key == key;
    }
}

fn inside(inner: [f32; 4], outer: [f32; 4]) -> bool {
    inner[0] >= outer[0] - 0.5
        && inner[1] >= outer[1] - 0.5
        && inner[0] + inner[2] <= outer[0] + outer[2] + 0.5
        && inner[1] + inner[3] <= outer[1] + outer[3] + 0.5
}

/// Select, switch tabs, order, open Build, pick, back out, hover: the dock,
/// the inspector's header and the hover card's header never move.
#[test]
fn nothing_docked_moves_while_you_play() {
    let sim = sim_at(&mods());
    let mut run = Run { ui: ui_for(&sim), cv: client(&sim), t: 0.0 };
    let founder = sim.world.colonists().next().unwrap();
    run.cv.selected = Some(founder);
    let c = sim.world.colony_center().unwrap();
    run.cv.hover_cell = Some(c);
    run.frame(&sim, Input::default());

    let dock = run.find("core:dock");
    let inspector = run.find("core:inspector")[1];
    let hover = run.find("core:hover")[1];
    let people = run.find("core:colonists")[1];
    let still = |run: &Run, step: &str| {
        assert_eq!(run.find("core:dock"), dock, "{step}: the dock moved");
        assert_eq!(run.find("core:inspector")[1], inspector, "{step}: the inspector's header moved");
        assert_eq!(run.find("core:hover")[1], hover, "{step}: the hover card's header moved");
        assert_eq!(run.find("core:colonists")[1], people, "{step}: the colonist list moved");
    };

    for tab in ["skills", "work", "overview"] {
        run.click(&sim, &format!("core:inspector.tabs.{tab}"));
        still(&run, &format!("the {tab} tab"));
    }

    run.cv.last_order = Some(("Arn will chop oak".into(), 0.5));
    run.frame(&sim, Input::default());
    assert!(inside(run.find("core:undo"), dock), "the undo chip sits in the dock bar");
    still(&run, "an order's undo chip");

    run.press(&sim, "b");
    let tray = run.find("core:dock.tray");
    assert!(tray[1] + tray[3] <= dock[1], "the tray floats above the dock: {tray:?}");
    let panel = run.find("core:inspector");
    assert!(tray[0] >= panel[0] + panel[2], "and clear of the inspector: {tray:?} vs {panel:?}");
    still(&run, "the Build tray");

    // Pick the wall; the client puts it in hand.
    let wall = run.cv.tools.iter().find(|t| t.key.ends_with(":wall")).expect("a wall tool").key.clone();
    run.click(&sim, &format!("core:toolbar.{wall}"));
    hand(&mut run.cv, &wall);
    run.frame(&sim, Input::default());
    assert!(run.ui.find("core:dock.tray").is_none(), "picking folds the tray");
    assert!(inside(run.find("core:dock.pill"), dock), "to the pill, in the dock bar");
    still(&run, "placing");

    run.press(&sim, "escape");
    hand(&mut run.cv, "select");
    run.frame(&sim, Input::default());
    assert!(run.ui.find("core:dock.tray").is_some(), "Escape brings the tray back");
    still(&run, "back to the tray");
    run.press(&sim, "escape");
    assert!(run.ui.find("core:dock.tray").is_none(), "a second Escape closes it");
    still(&run, "the tray closed");

    run.cv.last_order = None;
    run.frame(&sim, Input::default());
    still(&run, "the undo chip gone");

    // A busier cell: an oak, whose readout lists the tree.
    let oak = sim.world.defs.thing_id("tree_oak").unwrap();
    let tree = sim.world.ecs.query::<&Thing>().iter().find(|t| t.def == oak).map(|t| t.pos).expect("an oak");
    run.cv.hover_cell = Some(tree);
    run.frame(&sim, Input::default());
    still(&run, "hovering an oak");
}

/// A float panel is laid out over the map and never reflows the shell.
#[test]
fn a_float_panel_leaves_the_shell_alone() {
    let sim = sim_at(&mods());
    let mut run = Run { ui: ui_for(&sim), cv: client(&sim), t: 0.0 };
    run.frame(&sim, Input::default());
    let before: Vec<[f32; 4]> =
        ["core:topbar", "core:dock", "core:colonists", "core:inspector"].iter().map(|id| run.find(id)).collect();
    run.press(&sim, "b");
    let tray = run.find("core:dock.tray");
    let after: Vec<[f32; 4]> =
        ["core:topbar", "core:dock", "core:colonists", "core:inspector"].iter().map(|id| run.find(id)).collect();
    assert_eq!(before, after, "opening a tray moved a docked panel");
    let dock = run.find("core:dock");
    assert!((dock[1] - (tray[1] + tray[3]) - 8.0).abs() < 0.5, "the tray sits one panel gap over the dock: {tray:?}");
}

/// On a small screen a wide tray slides left to stay on screen, and still
/// moves nothing docked.
#[test]
fn a_wide_float_stays_on_a_small_screen() {
    let sim = sim_at(&mods());
    let mut run = Run { ui: ui_for(&sim), cv: client(&sim), t: 0.0 };
    run.cv.screen = (1024.0, 640.0);
    run.frame(&sim, Input::default());
    let dock = run.find("core:dock");
    let inspector = run.find("core:inspector");
    run.press(&sim, "b");
    let tray = run.find("core:dock.tray");
    assert!(tray[0] >= 0.0 && tray[0] + tray[2] <= 1024.0, "the tray is on screen: {tray:?}");
    assert_eq!(run.find("core:dock"), dock);
    assert_eq!(run.find("core:inspector"), inspector);
}
