//! The Work Board: priorities painted, not typed, with the rules and the
//! demand on show (DESIGN.md §4d).

mod common;

use common::*;
use rim_sim::{Command, Sim};
use rim_ui::view::{ClientView, UiAction};
use rim_ui::{Input, Ui};

/// The Work Board with `n` colonists, open and built.
fn board(n: usize) -> (Sim, Ui, ClientView) {
    let mut sim = sim_at(&mods());
    let human = sim.world.defs.creature_id("human").unwrap();
    let c = sim.world.colony_center().unwrap();
    for i in 1..n {
        sim.world.spawn_pawn(human, rim_sim::world::Faction::Player, c.offset(i as i32 % 5, i as i32 / 5), None);
    }
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    // A frame first, so the window opens on a screen of known size.
    frame(&mut ui, &sim, &cv, Default::default());
    ui.open_window("core:work");
    frame(&mut ui, &sim, &cv, Input { time: 0.5, ..Default::default() });
    frame(&mut ui, &sim, &cv, Input { time: 0.6, ..Default::default() });
    (sim, ui, cv)
}

/// The centre of a cell, from the sizes in mods/core/ui/work.luau.
fn cell_at(ui: &Ui, id: &str, r: usize, c: usize) -> (f32, f32) {
    let g = ui.find(id).unwrap_or_else(|| panic!("no {id}"));
    let (w, h) = if id == "core:work.names" { (110.0, 20.0) } else { (56.0, 20.0) };
    (g[0] + (c as f32 - 1.0) * (w + 2.0) + w / 2.0, g[1] + (r as f32 - 1.0) * (h + 2.0) + h / 2.0)
}

/// A work type's column, 1-based, in tie-break order.
fn column(sim: &Sim, id: &str) -> usize {
    let d = &sim.world.defs;
    d.work_order.iter().position(|&w| d.work_types[w as usize].id == id).unwrap() + 1
}

/// A press on a cell with no brush steps it: core's default is 3 of 4.
#[test]
fn a_cell_steps_a_priority() {
    let (sim, mut ui, mut cv) = board(1);
    let pawn = sim.world.colonists().next().unwrap();
    let at = cell_at(&ui, "core:work.grid", 1, column(&sim, "core:build"));
    let actions = click(&mut ui, &sim, &mut cv, at);
    assert_eq!(actions, vec![UiAction::SetPriority(pawn, "core:build".into(), 4)]);
}

/// One drag paints a stroke: every cell it enters gets the brush, each a
/// command, thirty colonists in one stroke.
#[test]
fn a_drag_paints_a_column_of_thirty_in_one_stroke() {
    let (sim, mut ui, mut cv) = board(30);
    let brush = ui.find("core:work.brush.1").expect("a brush per level");
    click(&mut ui, &sim, &mut cv, centre(brush));
    frame(&mut ui, &sim, &cv, Input { time: 5.0, ..Default::default() });
    let c = column(&sim, "core:haul");
    let mut actions = Vec::new();
    let mut t = 6.0;
    let mut step = |ui: &mut Ui, input: Input| {
        t += 0.01;
        actions.extend(frame(ui, &sim, &cv, Input { time: t, ..input }).actions);
    };
    let from = cell_at(&ui, "core:work.grid", 1, c);
    step(&mut ui, Input { mouse: from, ..Default::default() });
    step(&mut ui, Input { mouse: from, left_pressed: true, ..Default::default() });
    for r in 2..=30 {
        let at = cell_at(&ui, "core:work.grid", r, c);
        step(&mut ui, Input { mouse: at, ..Default::default() });
    }
    let end = cell_at(&ui, "core:work.grid", 30, c);
    let win = ui.find("core:work.window").expect("the window");
    assert!(end.1 < win[1] + win[3], "thirty rows fit the window, so the stroke is all visible cells");
    step(&mut ui, Input { mouse: end, left_released: true, ..Default::default() });
    let want: Vec<UiAction> = sim.world.colonists().map(|p| UiAction::SetPriority(p, "core:haul".into(), 1)).collect();
    assert_eq!(want.len(), 30);
    assert_eq!(actions, want, "one command per cell, each colonist once, in order");
}

/// A drag past the window's edge paints nothing the player can't see.
#[test]
fn a_drag_off_the_window_paints_nothing_hidden() {
    let (sim, mut ui, cv) = board(3);
    let c = column(&sim, "core:haul");
    let from = cell_at(&ui, "core:work.grid", 1, c);
    let win = ui.find("core:work.window").unwrap();
    let below = (from.0, win[1] + win[3] + 30.0);
    let mut actions = Vec::new();
    for (i, input) in [
        Input { mouse: from, ..Default::default() },
        Input { mouse: from, left_pressed: true, ..Default::default() },
        Input { mouse: below, ..Default::default() },
        Input { mouse: below, left_released: true, ..Default::default() },
    ]
    .into_iter()
    .enumerate()
    {
        actions.extend(frame(&mut ui, &sim, &cv, Input { time: 1.0 + i as f64 * 0.01, ..input }).actions);
    }
    assert_eq!(actions.len(), 1, "only the pressed cell: {actions:?}");
}

/// A trackpad's fractions add up to whole steps: one flick is one nudge.
#[test]
fn trackpad_fractions_make_whole_steps() {
    let (sim, mut ui, cv) = board(1);
    let at = cell_at(&ui, "core:work.grid", 1, column(&sim, "core:chop"));
    let mut actions = Vec::new();
    for i in 0..5 {
        actions.extend(
            frame(
                &mut ui,
                &sim,
                &cv,
                Input { mouse: at, wheel: 0.25, time: 1.0 + i as f64 * 0.01, ..Default::default() },
            )
            .actions,
        );
    }
    assert_eq!(actions.len(), 1, "1.25 notches is one step: {actions:?}");
}

/// The wheel over the names scrolls the board; over a cell it nudges.
#[test]
fn the_wheel_scrolls_the_board_away_from_the_cells() {
    let (sim, mut ui, cv) = board(60);
    let name = cell_at(&ui, "core:work.names", 3, 1);
    assert_eq!(ui.scroll_offset("core:work.body"), Some(0.0));
    let out = frame(&mut ui, &sim, &cv, Input { mouse: name, wheel: -1.0, time: 1.0, ..Default::default() });
    assert!(out.actions.is_empty() && out.captured_wheel);
    assert!(ui.scroll_offset("core:work.body").unwrap() > 0.0, "sixty rows scroll");
}

/// The wheel nudges a cell; shift-wheel nudges the whole column.
#[test]
fn the_wheel_nudges_a_cell_and_shift_a_column() {
    let (sim, mut ui, cv) = board(3);
    let c = column(&sim, "core:chop");
    let at = cell_at(&ui, "core:work.grid", 2, c);
    let one = frame(&mut ui, &sim, &cv, Input { mouse: at, wheel: 1.0, time: 1.0, ..Default::default() }).actions;
    let second = sim.world.colonists().nth(1).unwrap();
    assert_eq!(one, vec![UiAction::SetPriority(second, "core:chop".into(), 2)], "up is sooner");
    let all = frame(&mut ui, &sim, &cv, Input { mouse: at, wheel: -1.0, shift: true, time: 2.0, ..Default::default() })
        .actions;
    assert_eq!(all.len(), 3, "every colonist in the column: {all:?}");
    assert!(all.iter().all(|a| matches!(a, UiAction::SetPriority(_, w, 4) if w == "core:chop")));
}

/// The stance bar switches the colony's stance.
#[test]
fn the_stance_bar_switches() {
    let (sim, mut ui, mut cv) = board(1);
    let button = ui.find("core:work.stance.core:siege").expect("a button per stance");
    let actions = click(&mut ui, &sim, &mut cv, centre(button));
    assert_eq!(actions, vec![UiAction::SetStance("core:siege".into())]);
}

/// A cell a stance moves reads `base→effective`, and says why.
#[test]
fn cells_show_the_rules_and_say_why() {
    let (mut sim, mut ui, cv) = board(1);
    let siege = sim.world.defs.lookup("stance", "core:siege").unwrap();
    sim.push(Command::SetStance { stance: siege });
    sim.step();
    frame(&mut ui, &sim, &cv, Input { time: 10.0, ..Default::default() });
    let build = ui.grid_cell("core:work.grid", 1, column(&sim, "core:build")).unwrap();
    assert_eq!(build.text, "3→1");
    assert!(build.tip.as_deref().is_some_and(|t| t.starts_with("Build 1 = base 3, Siege -2")), "{:?}", build.tip);
    assert!(build.bar > 0.0, "the colonist's construction skill as a bar");
    assert_eq!(ui.grid_cell("core:work.grid", 1, column(&sim, "core:hunt")).unwrap().text, "3→–");
}

/// Column headers count the work waiting, and mark a column with work and
/// nobody on it at a high priority.
#[test]
fn headers_show_demand_and_mark_neglected_work() {
    let (mut sim, mut ui, cv) = board(1);
    let c = sim.world.colony_center().unwrap();
    let (wall, wood) = (sim.world.defs.thing_id("wall").unwrap(), sim.world.defs.thing_id("wood").unwrap());
    sim.push(Command::Build { thing: wall, stuff: Some(wood), a: c.offset(-6, 6), b: c.offset(-4, 6) });
    sim.step();
    frame(&mut ui, &sim, &cv, Input { time: 10.0, ..Default::default() });
    let col = column(&sim, "core:build");
    let bw = sim.world.defs.lookup("work_type", "core:build").unwrap();
    let waiting = rim_sim::ai::work_waiting(&sim.world)[bw as usize];
    assert!(waiting > 0);
    let head = ui.grid_cell("core:work.header", 2, col).unwrap();
    assert_eq!(head.text, waiting.to_string(), "the header is the sim's count");
    assert!(head.bg.is_some(), "build waits and the colonist has it at 3 of 4: marked");
    let pawn = sim.world.colonists().next().unwrap();
    sim.push(Command::SetPriority { pawn, work: bw, level: 1 });
    sim.step();
    frame(&mut ui, &sim, &cv, Input { time: 20.0, ..Default::default() });
    assert!(ui.grid_cell("core:work.header", 2, col).unwrap().bg.is_none(), "someone's on it now");
}

/// The ranked view writes the same numbers the board reads: a change in
/// either shows in the other.
#[test]
fn the_ranked_view_and_the_board_agree() {
    let (mut sim, mut ui, mut cv) = board(1);
    let pawn = sim.world.colonists().next().unwrap();
    let name = cell_at(&ui, "core:work.names", 1, 1);
    click(&mut ui, &sim, &mut cv, name);
    frame(&mut ui, &sim, &cv, Input { time: 5.0, ..Default::default() });
    let up = ui.find("core:work.ranked.core:mine.up").expect("the ranked view opens on the name");
    let actions = click(&mut ui, &sim, &mut cv, centre(up));
    assert_eq!(actions, vec![UiAction::SetPriority(pawn, "core:mine".into(), 2)]);
    // Applied as the client would, both read it back.
    let mine = sim.world.defs.lookup("work_type", "core:mine").unwrap();
    sim.push(Command::SetPriority { pawn, work: mine, level: 2 });
    sim.step();
    frame(&mut ui, &sim, &cv, Input { time: 20.0, ..Default::default() });
    assert_eq!(ui.grid_cell("core:work.grid", 1, column(&sim, "core:mine")).unwrap().text, "2");
    let tree = ui.snapshot();
    let level2 = tree.find("\"Level 2\"").expect("a level 2 group");
    let mine_row = tree.find("#core:work.ranked.core:mine").unwrap();
    assert!(mine_row > level2, "mine moved up into level 2:\n{tree}");

    // And the board's paint shows in the ranked view.
    let build = sim.world.defs.lookup("work_type", "core:build").unwrap();
    sim.push(Command::SetPriority { pawn, work: build, level: 0 });
    sim.step();
    frame(&mut ui, &sim, &cv, Input { time: 30.0, ..Default::default() });
    let tree = ui.snapshot();
    let never = tree.find("\"Never\"").expect("a never group");
    assert!(tree.find("#core:work.ranked.core:build").unwrap() > never, "build is under Never");
}

/// A mod's work type is a column with no UI change.
#[test]
fn a_mods_work_type_is_a_column() {
    let dir = scratch_mods(
        "boardcol",
        &[("tailor", "", &[("defs/work.toml", "[[work_type]]\nid = \"tailor\"\nlabel = \"Tailor\"\norder = 70\n")])],
    );
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    ui.open_window("core:work");
    frame(&mut ui, &sim, &cv, Default::default());
    let c = column(&sim, "tailor:tailor");
    assert_eq!(ui.grid_cell("core:work.header", 1, c).unwrap().text, "Tailor");
    assert_eq!(ui.grid_cell("core:work.grid", 1, c).unwrap().text, "3");
    let _ = std::fs::remove_dir_all(&dir);
}
