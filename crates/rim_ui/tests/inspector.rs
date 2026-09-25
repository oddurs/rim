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

/// Core plus a UI-only mod whose script is `ui`.
fn image_free_mods(name: &str, ui: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rim-ui-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy(&mods().join("core"), &dir.join("core"));
    let m = dir.join("probe");
    std::fs::create_dir_all(m.join("ui")).unwrap();
    std::fs::write(
        m.join("mod.toml"),
        "id = \"probe\"\nname = \"probe\"\nversion = \"0.1.0\"\napi = \"0.4\"\nui_api = \"0.3\"\ndepends = [\"core\"]\n",
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
