//! The inspector shows the selected thing as well as the selected pawn, and
//! mods add to it.

mod common;

use common::*;
use rim_sim::hecs::Entity;
use rim_sim::world::Thing;
use rim_sim::Command;
use rim_ui::Input;

/// The oak nearest the colony.
fn an_oak(sim: &rim_sim::Sim) -> (Entity, rim_sim::IVec) {
    let oak = sim.world.defs.thing_id("tree_oak").unwrap();
    let c = sim.world.colony_center().unwrap();
    sim.world
        .ecs
        .query::<(Entity, &Thing)>()
        .iter()
        .filter(|(_, t)| t.def == oak)
        .map(|(e, t)| (e, t.pos))
        .min_by_key(|(e, p)| (p.octile(c), e.id()))
        .expect("an oak")
}

#[test]
fn a_selected_thing_shows_in_the_inspector() {
    let mut sim = sim_at(&mods());
    let (tree, at) = an_oak(&sim);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.selected = Some(tree);
    frame(&mut ui, &sim, &cv, Input::default());
    assert!(ui.find("core:inspector.panel").is_some(), "the thing panel");
    assert!(ui.find("core:inspector.thing").is_some(), "the extension point");
    assert!(ui.find("core:inspector.why").is_none(), "nothing is blocked: it isn't designated");

    // Designated but out of every colonist's reach: it says so.
    let chop = sim.world.defs.lookup("designation", "core:chop").unwrap();
    sim.push(Command::Designate { designation: chop, a: at, b: at });
    sim.step();
    for c in sim.world.colonists().collect::<Vec<_>>() {
        let _ = sim.world.ecs.despawn(c);
    }
    sim.world.pawns.clear();
    // The world changed but the client didn't: the tree rebuilds on its clock.
    frame(&mut ui, &sim, &cv, Input { time: 1.0, ..Default::default() });
    assert!(ui.find("core:inspector.why").is_some(), "why its work isn't happening");

    // Nothing selected, no panel.
    cv.selected = None;
    frame(&mut ui, &sim, &cv, Input { time: 2.0, ..Default::default() });
    assert!(ui.find("core:inspector.panel").is_none());
}

#[test]
fn a_selected_pawn_still_gets_the_pawn_panel() {
    let sim = sim_at(&mods());
    let (tree, _) = an_oak(&sim);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.selected = Some(tree);
    frame(&mut ui, &sim, &cv, Input::default());
    cv.selected = sim.world.colonists().next();
    frame(&mut ui, &sim, &cv, Input::default());
    assert!(ui.find("core:inspector.bars").is_some(), "the pawn panel");
    assert!(ui.find("core:inspector.thing").is_none(), "not the thing panel");
}

#[test]
fn a_mod_extends_the_thing_inspector() {
    let extra = r#"
ui.extend("core:inspector.thing", function(view)
    local th = view.thing(view.selected())
    return ui.text({ id = "probe:hp", string.format("%d of %d", th.hp, th.max_hp) })
end)
"#;
    let dir = image_free_mods("inspector-extend", extra);
    let sim = sim_at(&dir);
    let (tree, _) = an_oak(&sim);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.selected = Some(tree);
    frame(&mut ui, &sim, &cv, Input::default());
    assert!(ui.find("probe:hp").is_some(), "the mod's section: {:?}", ui.warnings());
    let _ = std::fs::remove_dir_all(dir);
}

/// The thing slot exists only while a thing is selected. Extending it with
/// nothing selected is no mistake, so it isn't reported as one.
#[test]
fn extending_a_slot_that_is_only_sometimes_there_is_no_warning() {
    let extra = r#"ui.extend("core:inspector.thing", function(view) return nil end)"#;
    let dir = image_free_mods("inspector-sometimes", extra);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.selected = None;
    frame(&mut ui, &sim, &cv, Input::default());
    assert!(ui.find("core:inspector.thing").is_none(), "the slot isn't there");
    assert!(ui.warnings().iter().all(|w| !w.contains("core:inspector.thing")), "{:?}", ui.warnings());
    let _ = std::fs::remove_dir_all(dir);
}

/// Core plus a UI-only mod whose script is `ui`.
fn image_free_mods(name: &str, ui: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rim-ui-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy(&mods().join("core"), &dir.join("core"));
    let m = dir.join("probe");
    std::fs::create_dir_all(m.join("ui")).unwrap();
    std::fs::write(
        m.join("mod.toml"),
        format!(
            "id = \"probe\"\nname = \"probe\"\nversion = \"0.1.0\"\napi = \"{}.{}\"\nui_api = \"{}.{}\"\ndepends = [\"core\"]\n",
            rim_sim::API_VERSION.0,
            rim_sim::API_VERSION.1,
            rim_ui::api::UI_API_VERSION.0,
            rim_ui::api::UI_API_VERSION.1
        ),
    )
    .unwrap();
    std::fs::write(m.join("ui/probe.luau"), ui).unwrap();
    dir
}

fn copy(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

/// A colonist's panel has tabs: needs and gear, every skill with its level
/// and what trains it, and the work they'd do with rules applied.
#[test]
fn a_colonist_panel_has_overview_skills_and_work_tabs() {
    let mut sim = sim_at(&mods());
    let founder = sim.world.colonists().next().unwrap();
    let wood = sim.world.defs.thing_id("core:wood").unwrap();
    sim.world.ecs.get::<&mut rim_sim::world::Pawn>(founder).unwrap().carry = Some(rim_sim::world::Lot::new(wood, 7));
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.selected = Some(founder);
    frame(&mut ui, &sim, &cv, Input::default());
    assert!(ui.find("core:inspector.bars").is_some(), "overview first");
    assert!(ui.snapshot().contains("carrying wood ×7"), "{}", ui.snapshot());

    let tab = ui.find("core:inspector.tabs.skills").expect("a skills tab");
    click(&mut ui, &sim, &mut cv, centre(tab));
    let tree = ui.snapshot();
    for s in &sim.world.defs.skills {
        assert!(ui.find(&format!("core:inspector.skill.{}", s.id)).is_some(), "{}: {tree}", s.id);
    }
    assert_eq!(tip(&mut ui, &sim, &mut cv, "core:inspector.skill.core:plants", 1.0), "Trained by Chop, Harvest");

    let tab = ui.find("core:inspector.tabs.work").expect("a work tab");
    click(&mut ui, &sim, &mut cv, centre(tab));
    assert!(ui.find("core:inspector.work.core:build").is_some());
    let siege = sim.world.defs.lookup("stance", "core:siege").unwrap();
    sim.push(Command::SetStance { stance: siege });
    sim.step();
    frame(&mut ui, &sim, &cv, Input { time: 10.0, ..Default::default() });
    assert_eq!(tip(&mut ui, &sim, &mut cv, "core:inspector.work.core:build", 10.0), "Build 1 = base 3, Siege -2");

    // Anyone else's panel has no tabs.
    cv.selected = sim.world.pawns.iter().copied().find(|&e| e != founder);
    frame(&mut ui, &sim, &cv, Input { time: 11.0, ..Default::default() });
    if cv.selected.is_some_and(|e| !sim.world.colonists().any(|c| c == e)) {
        assert!(ui.find("core:inspector.tabs").is_none());
    }
}

/// Hover a node until its tooltip shows, and read the tooltip's text.
fn tip(ui: &mut rim_ui::Ui, sim: &rim_sim::Sim, cv: &mut rim_ui::view::ClientView, id: &str, now: f64) -> String {
    let at = centre(ui.find(id).unwrap_or_else(|| panic!("no {id}")));
    cv.mouse = at;
    frame(ui, sim, cv, Input { mouse: at, time: now, ..Default::default() });
    frame(ui, sim, cv, Input { mouse: at, time: now + 5.0, ..Default::default() });
    ui.tooltip_text(now + 5.0).unwrap_or_else(|| panic!("no tooltip for {id}"))
}
