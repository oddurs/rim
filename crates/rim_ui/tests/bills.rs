//! The crafting plugin's bills panel: a station's section in the thing
//! inspector, and the commands its buttons send.

mod common;

use common::*;
use rim_sim::{Command, IVec, Sim};
use rim_ui::view::UiAction;
use rim_ui::Input;

const KIT: &str = r##"
[[thing]]
id = "chip"
label = "chip"
color = "#445566"
category = "item"
look.layers = [{ draw = "fill" }]
tags = ["shard"]

[[crafting.recipe]]
id = "blade"
label = "blade"
station = "crafting:hand"
inputs = [{ tag = "shard", count = 2 }]
outputs = [{ thing = "chip" }]
work = 60
"##;

/// Core, crafting and a mod with one recipe; a finished crafting spot by
/// the colony, and time for the plugin to publish its catalog.
fn world(name: &str) -> (Sim, rim_sim::hecs::Entity, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("rim-ui-bills-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&mods().join("core"), &dir.join("core"));
    copy_dir(&mods().join("crafting"), &dir.join("crafting"));
    std::fs::create_dir_all(dir.join("kit/defs")).unwrap();
    std::fs::write(
        dir.join("kit/mod.toml"),
        "id = \"kit\"\nname = \"kit\"\nversion = \"0.1.0\"\napi = \"0.6\"\ndepends = [\"core\", \"crafting\"]\n",
    )
    .unwrap();
    std::fs::write(dir.join("kit/defs/kit.toml"), KIT).unwrap();
    let mut sim = sim_at(&dir);
    sim.step();
    let c = sim.world.colony_center().unwrap();
    let at = (1..12)
        .flat_map(|d| [c.offset(d, 0), c.offset(-d, 0), c.offset(0, d), c.offset(0, -d)])
        .find(|&p: &IVec| sim.world.map.passable(p) && sim.world.map.fixture_at(p).is_none())
        .expect("room for a spot");
    let spot = sim.world.defs.thing_id("crafting:spot").unwrap();
    let e = sim.world.spawn_fixture_of(spot, at, false, None).unwrap();
    rim_sim::ai::complete_building(&mut sim.world, e);
    for _ in 0..120 {
        sim.step();
    }
    (sim, e, dir)
}

#[test]
fn a_station_shows_its_bills_and_adds_one() {
    let (mut sim, spot, dir) = world("add");
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.selected = Some(spot);
    frame(&mut ui, &sim, &cv, Input::default());
    assert!(ui.find("crafting:bills").is_some(), "the bills section: {:?}", ui.warnings());
    let add = ui.find("crafting:add.kit:blade").expect("a button for the recipe");

    let sent: Vec<_> = click(&mut ui, &sim, &mut cv, centre(add))
        .into_iter()
        .filter_map(|a| match a {
            UiAction::Send(name, data) => Some((name, data)),
            _ => None,
        })
        .collect();
    assert_eq!(sent.len(), 1, "one command");
    assert_eq!(sent[0].0, "crafting:add_bill");
    for (name, data) in sent {
        sim.push(Command::ModEvent { name, data });
    }
    for _ in 0..120 {
        sim.step();
    }
    frame(&mut ui, &sim, &cv, Input { time: 1.0, ..Default::default() });
    assert!(ui.find("crafting:bill.1").is_some(), "the new bill");
    assert!(ui.find("crafting:bill.1.why").is_some(), "and what it's waiting on");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_thing_that_makes_nothing_has_no_bills() {
    let (sim, _, dir) = world("none");
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    let oak = sim.world.defs.thing_id("core:tree_oak").unwrap();
    cv.selected = sim
        .world
        .ecs
        .query::<(rim_sim::hecs::Entity, &rim_sim::world::Thing)>()
        .iter()
        .filter(|(_, t)| t.def == oak)
        .map(|(e, _)| e)
        .min_by_key(|e| e.id());
    frame(&mut ui, &sim, &cv, Input::default());
    assert!(ui.find("core:inspector.thing").is_some());
    assert!(ui.find("crafting:bills").is_none());
    let _ = std::fs::remove_dir_all(dir);
}
