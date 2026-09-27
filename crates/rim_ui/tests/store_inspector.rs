//! The store inspector (DESIGN.md §4f, Showing contents): a selected
//! stockpile or container shows Contents and Accepts tabs, the tabs send
//! the store's filter commands, and a mod adds a sort order.

mod common;

use common::*;
use rim_sim::hecs::Entity;
use rim_sim::world::Lot;
use rim_sim::zone::StoreRef;
use rim_sim::Command;
use rim_ui::view::{UiAction, UiFilterEdit};
use rim_ui::Input;

/// A stockpile three cells wide near the colony, with stone in it.
fn stockpile(sim: &mut rim_sim::Sim) -> u32 {
    let c = sim.world.colony_center().unwrap();
    let stone = sim.world.defs.thing_id("stone").unwrap();
    let open = |p| sim.world.room_for(stone, None, p) > 0;
    let at =
        (3..30).map(|r| c.offset(r, r)).find(|&p| open(p) && open(p.offset(1, 0)) && open(p.offset(2, 0))).unwrap();
    sim.push(Command::Stockpile { a: at, b: at.offset(2, 0), zone: None });
    sim.step();
    sim.world.put_lot(Lot::new(stone, 30), at);
    sim.world.zones.list[0].id
}

const CRATE: &str = r##"
[[thing]]
id = "crate"
label = "crate"
color = "#b98a55"
category = "building"
blocks = true
store = { slots = 4, accepts = { not_tags = ["bulky"] } }
"##;

/// Core plus a mod with a crate, and a UI mod file `ui` (may be empty).
fn crate_mods(name: &str, ui: &str) -> std::path::PathBuf {
    let files: Vec<(&str, &str)> = if ui.is_empty() {
        vec![("defs/crate.toml", CRATE)]
    } else {
        vec![("defs/crate.toml", CRATE), ("ui/probe.luau", ui)]
    };
    scratch_mods(name, &[("probe", "", &files)])
}

fn a_crate(sim: &mut rim_sim::Sim) -> Entity {
    let def = sim.world.defs.thing_id("probe:crate").unwrap();
    let c = sim.world.colony_center().unwrap();
    let berries = sim.world.defs.thing_id("berries").unwrap();
    let at = (3..30).map(|r| c.offset(-r, r)).find(|&p| sim.world.room_for(berries, None, p) > 0).unwrap();
    let e = sim.world.spawn_fixture_of(def, at, false, None).expect("placed");
    sim.world.put_in_store(e, Lot::new(berries, 20));
    e
}

#[test]
fn a_selected_stockpile_shows_its_contents_and_what_it_takes() {
    let mut sim = sim_at(&mods());
    let z = stockpile(&mut sim);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    // As the client does on selecting a stockpile: nothing else selected.
    cv.selected = None;
    cv.selected_zone = Some(z);
    frame(&mut ui, &sim, &cv, Input::default());
    assert!(ui.find("core:inspector.panel").is_some(), "{:?}\n{}", ui.warnings(), ui.snapshot());
    assert!(ui.find("core:inspector.tabs.contents").is_some() && ui.find("core:inspector.tabs.accepts").is_some());
    assert!(ui.find("core:store.contents").is_some(), "contents first");
    let stone = ui.grid_cell("core:store.grid.1", 1, 1).expect("a token for the stone");
    assert!(stone.token.is_some(), "drawn as a token");
    // To the Accepts tab: the tree, and the summary says everything.
    let tab = ui.find("core:inspector.tabs.accepts").unwrap();
    click(&mut ui, &sim, &mut cv, centre(tab));
    frame(&mut ui, &sim, &cv, Input { time: 1.0, ..Default::default() });
    assert!(ui.find("core:store.accepts").is_some());
    assert!(ui.snapshot().contains("Everything"), "{}", ui.snapshot());
}

#[test]
fn a_selected_container_shows_its_slots() {
    let dir = crate_mods("store-inspector-crate", "");
    let mut sim = sim_at(&dir);
    let e = a_crate(&mut sim);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.selected = Some(e);
    frame(&mut ui, &sim, &cv, Input::default());
    assert!(
        ui.find("core:inspector.tabs.contents").is_some() && ui.find("core:inspector.tabs.accepts").is_some(),
        "{:?}",
        ui.warnings()
    );
    assert!(ui.snapshot().contains("1 of 4 slots"), "{}", ui.snapshot());
    // The three empty slots, dashed, after what it holds.
    assert!(ui.grid_cell("core:store.grid.empty", 1, 3).is_some_and(|c| c.token.is_some()));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_accepts_tab_sends_the_stores_filter_commands() {
    let mut sim = sim_at(&mods());
    let z = stockpile(&mut sim);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    // As the client does on selecting a stockpile: nothing else selected.
    cv.selected = None;
    cv.selected_zone = Some(z);
    frame(&mut ui, &sim, &cv, Input::default());
    let tab = ui.find("core:inspector.tabs.accepts").unwrap();
    click(&mut ui, &sim, &mut cv, centre(tab));
    frame(&mut ui, &sim, &cv, Input { time: 1.0, ..Default::default() });
    let food = ui.find("core:store.take.core:food").expect("the Food row");
    let sent = click(&mut ui, &sim, &mut cv, centre(food));
    assert!(
        sent.contains(&UiAction::StoreFilter(StoreRef::Zone(z), UiFilterEdit::Category("core:food".into(), false))),
        "{sent:?}"
    );
    let half = ui.find("core:store.condition.50").unwrap();
    let sent = click(&mut ui, &sim, &mut cv, centre(half));
    assert!(sent.contains(&UiAction::StoreFilter(StoreRef::Zone(z), UiFilterEdit::Condition(50, 100))), "{sent:?}");
}

#[test]
fn a_mod_adds_a_sort_order() {
    let probe = r#"
local storage = require("@core/ui/storage")
storage.sorter({ id = "probe:backwards", label = "Backwards", order = 5, key = function(r) return -(string.byte(r.label or "a")) end })
"#;
    let dir = crate_mods("store-inspector-sorter", probe);
    let mut sim = sim_at(&dir);
    let e = a_crate(&mut sim);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.selected = Some(e);
    frame(&mut ui, &sim, &cv, Input::default());
    // Cycling the sort control reaches the mod's order.
    let mut seen = false;
    for i in 0..6 {
        let sort = ui.find("core:store.sort").expect("the sort control");
        click(&mut ui, &sim, &mut cv, centre(sort));
        frame(&mut ui, &sim, &cv, Input { time: 1.0 + i as f64, ..Default::default() });
        seen |= ui.snapshot().contains("Sort: Backwards");
    }
    assert!(seen, "{}", ui.snapshot());
    let _ = std::fs::remove_dir_all(dir);
}
