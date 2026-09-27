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
    // Hand sets nothing, so cells read as the work types' defaults.
    hands(&mut sim);
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
    assert!(
        build.tip.as_deref().is_some_and(|t| t.starts_with("Build: First = default Later · Siege −2")),
        "{:?}",
        build.tip
    );
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

/// The shelves write the same settings the board reads: a change in either
/// shows in the other.
#[test]
fn the_shelves_and_the_board_agree() {
    let (mut sim, mut ui, mut cv) = board(1);
    let pawn = sim.world.colonists().next().unwrap();
    let name = cell_at(&ui, "core:work.names", 1, 1);
    click(&mut ui, &sim, &mut cv, name);
    frame(&mut ui, &sim, &cv, Input { time: 5.0, ..Default::default() });
    let mine_token = ui.find("core:work.ranked.core:mine").expect("the shelves open on the name");
    let actions = click(&mut ui, &sim, &mut cv, centre(mine_token));
    assert_eq!(actions, vec![UiAction::SetPriority(pawn, "core:mine".into(), 4)], "a click moves it a shelf later");
    // Applied as the client would, both read it back.
    let mine = sim.world.defs.lookup("work_type", "core:mine").unwrap();
    sim.push(Command::SetPriority { pawn, work: mine, level: 4 });
    sim.step();
    frame(&mut ui, &sim, &cv, Input { time: 20.0, ..Default::default() });
    let tree = ui.snapshot();
    let spare = tree.find("#core:work.shelf.4").expect("a Spare time shelf");
    let mine_at = tree.find("#core:work.ranked.core:mine").unwrap();
    assert!(mine_at > spare && tree[spare..mine_at].contains("Spare time"), "mine is on the Spare time shelf:\n{tree}");
    let tab = ui.find("core:work.lens.core:work.board").unwrap();
    click(&mut ui, &sim, &mut cv, centre(tab));
    frame(&mut ui, &sim, &cv, Input { time: 25.0, ..Default::default() });
    assert_eq!(ui.grid_cell("core:work.grid", 1, column(&sim, "core:mine")).unwrap().text, "4", "and on the board");

    // And the board's paint shows on the shelves.
    let build = sim.world.defs.lookup("work_type", "core:build").unwrap();
    sim.push(Command::SetPriority { pawn, work: build, level: 0 });
    sim.step();
    let tab = ui.find("core:work.lens.core:work.person").unwrap();
    click(&mut ui, &sim, &mut cv, centre(tab));
    frame(&mut ui, &sim, &cv, Input { time: 30.0, ..Default::default() });
    let tree = ui.snapshot();
    let never = tree.find("#core:work.shelf.0").expect("a never shelf");
    assert!(tree.find("#core:work.ranked.core:build").unwrap() > never, "build is on the Never shelf");
}

/// A pinned cell is drawn as the player's, and clicking it back to what the
/// colonist would inherit hands it back rather than pinning the default.
#[test]
fn a_pin_is_marked_and_clicking_back_hands_it_back() {
    let (mut sim, mut ui, mut cv) = board(1);
    let pawn = sim.world.colonists().next().unwrap();
    let build = sim.world.defs.lookup("work_type", "core:build").unwrap();
    let c = column(&sim, "core:build");
    assert!(ui.grid_cell("core:work.grid", 1, c).unwrap().dot.is_none(), "a default isn't a pin");
    sim.push(Command::SetPriority { pawn, work: build, level: 2 });
    sim.step();
    frame(&mut ui, &sim, &cv, Input { time: 20.0, ..Default::default() });
    let cell = ui.grid_cell("core:work.grid", 1, c).unwrap();
    assert!(cell.dot.is_some() && cell.weight.is_some(), "a pin is bold with a dot: {cell:?}");
    assert!(cell.tip.as_deref().unwrap_or("").contains("setting Soon"), "the tip says whose: {:?}", cell.tip);
    let at = cell_at(&ui, "core:work.grid", 1, c);
    let actions = click(&mut ui, &sim, &mut cv, at);
    assert_eq!(
        actions,
        vec![UiAction::ClearPriority(pawn, "core:build".into())],
        "Soon steps to Later, core's default"
    );
}

/// Over a cell, a level's number sets it, N sets never and A hands it back,
/// ahead of any key binding.
#[test]
fn keys_over_a_cell_set_and_hand_back() {
    let (mut sim, mut ui, cv) = board(1);
    let pawn = sim.world.colonists().next().unwrap();
    let build = sim.world.defs.lookup("work_type", "core:build").unwrap();
    sim.push(Command::SetPriority { pawn, work: build, level: 2 });
    sim.step();
    let at = cell_at(&ui, "core:work.grid", 1, column(&sim, "core:build"));
    let mut t = 20.0;
    let mut press = |ui: &mut Ui, key: &str| {
        t += 0.1;
        frame(ui, &sim, &cv, Input { mouse: at, time: t, ..Default::default() });
        let out =
            frame(ui, &sim, &cv, Input { mouse: at, time: t + 0.05, pressed: vec![key.into()], ..Default::default() });
        assert!(out.captured_keys, "{key} is the cell's, not a binding's");
        out.actions
    };
    assert_eq!(press(&mut ui, "1"), vec![UiAction::SetPriority(pawn, "core:build".into(), 1)]);
    assert_eq!(press(&mut ui, "n"), vec![UiAction::SetPriority(pawn, "core:build".into(), 0)]);
    assert_eq!(press(&mut ui, "a"), vec![UiAction::ClearPriority(pawn, "core:build".into())]);
    assert_eq!(
        press(&mut ui, "3"),
        vec![UiAction::ClearPriority(pawn, "core:build".into())],
        "Later is what build inherits: its number hands the pin back"
    );
}

/// The brush speaks in the scale's names.
#[test]
fn the_brush_names_the_levels() {
    let (_sim, ui, _cv) = board(1);
    let tree = ui.snapshot();
    for name in ["First", "Soon", "Later", "Spare time", "Never"] {
        assert!(tree.contains(&format!("\"{name}\"")), "the brush offers {name}:\n{tree}");
    }
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

/// A mod's lens: a tab beside core's, a view over the same board, and its
/// writes are the board's commands.
#[test]
fn a_mods_lens_is_a_tab_that_writes_like_the_board() {
    const LENS: &str = r#"
local work = require("@core/ui/work")
local kit = require("@core/ui/kit")
work.lens({ id = "crews:waiting", label = "Waiting", order = 30, draw = function(_view, board)
    local items = {}
    for _, c in board.cols do
        table.insert(items, kit.button({ id = "crews:first." .. c.id, label = c.label .. " " .. c.waiting, on_click = function()
            act.set_priority(board.rows[1].id, c.id, 1)
        end }))
    end
    return kit.col({ gap = "xs" }, items)
end })
"#;
    let dir = scratch_mods("lens", &[("crews", "", &[("ui/lens.luau", LENS)])]);
    let sim = sim_at(&dir);
    let pawn = sim.world.colonists().next().unwrap();
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    ui.open_window("core:work");
    frame(&mut ui, &sim, &cv, Input { time: 0.5, ..Default::default() });
    for id in ["core:work.lens.core:work.board", "core:work.lens.core:work.person", "core:work.lens.crews:waiting"] {
        assert!(ui.find(id).is_some(), "a tab for {id}");
    }
    let tabs = ui.snapshot();
    let at = |id: &str| tabs.find(&format!("#{id}")).unwrap();
    assert!(at("core:work.lens.core:work.person") < at("core:work.lens.crews:waiting"), "tabs in order");
    assert!(ui.find("crews:first.core:haul").is_none(), "the board shows first");

    let tab = ui.find("core:work.lens.crews:waiting").unwrap();
    click(&mut ui, &sim, &mut cv, centre(tab));
    frame(&mut ui, &sim, &cv, Input { time: 5.0, ..Default::default() });
    assert!(ui.find("core:work.grid").is_none(), "one lens at a time");
    let haul = ui.find("crews:first.core:haul").expect("the mod's lens draws");
    let actions = click(&mut ui, &sim, &mut cv, centre(haul));
    assert_eq!(actions, vec![UiAction::SetPriority(pawn, "core:haul".into(), 1)]);
}

/// A name on the board opens the Person lens on that colonist, and the
/// separate ranked window is gone.
#[test]
fn a_name_opens_the_person_lens() {
    let (sim, mut ui, mut cv) = board(3);
    let second = sim.world.colonists().nth(1).unwrap();
    let name = cell_at(&ui, "core:work.names", 2, 1);
    click(&mut ui, &sim, &mut cv, name);
    frame(&mut ui, &sim, &cv, Input { time: 5.0, ..Default::default() });
    assert!(ui.find("core:work.grid").is_none(), "the board gave way");
    let picked = ui.find(&format!("core:work.person.{}", second.to_bits().get())).expect("a button per colonist");
    assert!(ui.snapshot().contains("core:work.shelf.1"), "their shelves");
    assert!(picked[2] > 0.0);
    assert!(!ui.is_open("core:work.ranked"), "no separate window");
}

/// Put a colonist in a role by label, as the client would.
fn assign(sim: &mut Sim, pawn: rim_sim::hecs::Entity, label: &str) -> u16 {
    let role = sim.world.work_roles.iter().position(|r| r.label == label).unwrap() as u16;
    sim.push(Command::AssignWorkRole { pawn, role });
    sim.step();
    role
}

/// Rows are grouped by role, with the role's name where its group starts.
#[test]
fn the_board_groups_rows_by_role() {
    let (mut sim, mut ui, cv) = board(3);
    let v: Vec<_> = sim.world.colonists().collect();
    assign(&mut sim, v[0], "Forager");
    assign(&mut sim, v[1], "Builder");
    frame(&mut ui, &sim, &cv, Input { time: 10.0, ..Default::default() });
    let texts: Vec<String> = (1..=3).map(|r| ui.grid_cell("core:work.rolecol", r, 1).unwrap().text).collect();
    let names: Vec<String> = (1..=3).map(|r| ui.grid_cell("core:work.names", r, 1).unwrap().text).collect();
    let name = |e| sim.world.ecs.get::<&rim_sim::world::Pawn>(e).unwrap().name.clone();
    assert_eq!(names, [name(v[2]), name(v[1]), name(v[0])], "in role order: the default first, then Builder, Forager");
    assert_eq!(texts[1], "Builder");
    assert_eq!(texts[2], "Forager");
}

/// In the Roles lens, a colonist is picked up and put in another role; a
/// role's token cycles the role's level for everyone in it.
#[test]
fn the_roles_lens_moves_colonists_and_edits_roles() {
    let (sim, mut ui, mut cv) = board(2);
    let pawn = sim.world.colonists().next().unwrap();
    let tab = ui.find("core:work.lens.core:work.roles").expect("a Roles tab");
    click(&mut ui, &sim, &mut cv, centre(tab));
    frame(&mut ui, &sim, &cv, Input { time: 5.0, ..Default::default() });
    let chip = ui.find(&format!("core:work.member.{}", pawn.to_bits().get())).expect("a chip per member");
    click(&mut ui, &sim, &mut cv, centre(chip));
    frame(&mut ui, &sim, &cv, Input { time: 6.0, ..Default::default() });
    let builder = sim.world.work_roles.iter().position(|r| r.label == "Builder").unwrap() as u16;
    let put = ui.find(&format!("core:work.role.put.{}", builder + 1)).expect("Put ... here on the other cards");
    let actions = click(&mut ui, &sim, &mut cv, centre(put));
    assert_eq!(actions, vec![UiAction::AssignWorkRole(pawn, builder)]);

    // Builder hauls Soon; a click moves it to Later, the default, so the
    // role leaves it to the default.
    let haul = ui.find(&format!("core:work.role.{}.core:haul", builder + 1)).unwrap();
    let actions = click(&mut ui, &sim, &mut cv, centre(haul));
    assert_eq!(actions, vec![UiAction::SetRolePriority(builder, "core:haul".into(), None)]);
    let mine = ui.find(&format!("core:work.role.{}.core:mine", builder + 1)).unwrap();
    let actions = click(&mut ui, &sim, &mut cv, centre(mine));
    assert_eq!(
        actions,
        vec![UiAction::SetRolePriority(builder, "core:mine".into(), None)],
        "Soon to Later: the default"
    );
}

/// Two colonists pinned into the same shape are offered a role of their
/// own, once.
#[test]
fn two_alike_are_offered_a_role() {
    let (mut sim, mut ui, mut cv) = board(3);
    let v: Vec<_> = sim.world.colonists().collect();
    let d = sim.world.defs.clone();
    for &e in &v[..2] {
        sim.push(Command::SetPriority { pawn: e, work: d.lookup("work_type", "core:build").unwrap(), level: 1 });
        sim.push(Command::SetPriority { pawn: e, work: d.lookup("work_type", "core:haul").unwrap(), level: 2 });
    }
    sim.step();
    frame(&mut ui, &sim, &cv, Input { time: 10.0, ..Default::default() });
    let make = ui.find("core:work.alike.make").expect("the offer");
    let n = sim.world.work_roles.len() as u16;
    let actions = click(&mut ui, &sim, &mut cv, centre(make));
    assert_eq!(actions.len(), 3, "{actions:?}");
    assert!(matches!(&actions[0], UiAction::CreateRoleFromPawn(_, e) if *e == v[0]));
    assert_eq!(&actions[1..], [UiAction::AssignWorkRole(v[0], n), UiAction::AssignWorkRole(v[1], n)]);

    let no = ui.find("core:work.alike.no").unwrap();
    click(&mut ui, &sim, &mut cv, centre(no));
    frame(&mut ui, &sim, &cv, Input { time: 20.0, ..Default::default() });
    assert!(ui.find("core:work.alike").is_none(), "not again after Not now");
}

/// The orders panel lists core's standing orders with their readings, and
/// its switch turns one off.
#[test]
fn the_orders_panel_lists_and_switches_orders() {
    let (mut sim, mut ui, mut cv) = board(1);
    sim.world.set_reading("core:food_days", 3.4);
    frame(&mut ui, &sim, &cv, Input { time: 5.0, ..Default::default() });
    let button = ui.find("core:work.orders").expect("an orders button beside the stances");
    assert!(ui.snapshot().contains("Orders · 1 on"), "{}", ui.snapshot());
    click(&mut ui, &sim, &mut cv, centre(button));
    frame(&mut ui, &sim, &cv, Input { time: 6.0, ..Default::default() });
    let tree = ui.snapshot();
    for want in
        ["Food is low", "Harvest and Hunt one level sooner", "on under 5, off at 8", "on · 3.4", "Wood before winter"]
    {
        assert!(tree.contains(want), "the panel says {want}:\n{tree}");
    }
    let switch = ui.find("core:work.order.core:food_low.switch").unwrap();
    let actions = click(&mut ui, &sim, &mut cv, centre(switch));
    assert_eq!(actions, vec![UiAction::SetRuleEnabled("core:food_low".into(), false)]);
}

/// Focus stays lit on the HUD: a stance that isn't Normal, or an order
/// acting, and nothing when neither.
#[test]
fn focus_shows_on_the_hud_only_when_it_acts() {
    let (mut sim, mut ui, cv) = board(1);
    for id in ["core:food_low", "core:loose_items", "core:wood_for_winter"] {
        let rule = sim.world.defs.lookup("priority_rule", id).unwrap();
        sim.push(Command::SetRuleEnabled { rule, on: false });
    }
    sim.step();
    frame(&mut ui, &sim, &cv, Input { time: 5.0, ..Default::default() });
    assert!(ui.find("core:focus.hud").is_none(), "Normal, no orders: nothing to show");
    sim.push(Command::SetStance { stance: sim.world.defs.lookup("stance", "core:siege").unwrap() });
    sim.step();
    frame(&mut ui, &sim, &cv, Input { time: 6.0, ..Default::default() });
    assert!(ui.find("core:focus.hud").is_some(), "Siege is lit");
    assert!(ui.snapshot().contains("\"Siege\""));
}

/// The board says what Focus is doing to it.
#[test]
fn the_board_says_what_focus_moves() {
    let (mut sim, mut ui, cv) = board(2);
    sim.push(Command::SetStance { stance: sim.world.defs.lookup("stance", "core:harvest").unwrap() });
    sim.step();
    frame(&mut ui, &sim, &cv, Input { time: 5.0, ..Default::default() });
    let tree = ui.snapshot();
    assert!(ui.find("core:work.focus").is_some(), "a focus line");
    assert!(tree.contains("Focus: Harvest") && tree.contains("settings"), "{tree}");
}
