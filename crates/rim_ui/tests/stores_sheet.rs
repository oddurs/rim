//! What the colony has, across every store (DESIGN.md §4f): view.stock reads
//! the sim's stock ledger, the Stores sheet shows it, and a card shows the
//! store the cursor rests on.

mod common;

use common::*;
use rim_sim::world::Lot;
use rim_sim::Command;
use rim_ui::Input;

/// A stockpile of three cells near the colony, with stone in one and wood
/// in another; stone also lies loose outside it.
fn stockpile(sim: &mut rim_sim::Sim) -> (u32, rim_sim::IVec) {
    let c = sim.world.colony_center().unwrap();
    let (stone, wood) = (sim.world.defs.thing_id("stone").unwrap(), sim.world.defs.thing_id("wood").unwrap());
    let open = |p| sim.world.room_for(stone, None, p) > 0;
    let at =
        (3..30).map(|r| c.offset(r, r)).find(|&p| open(p) && open(p.offset(1, 0)) && open(p.offset(2, 0))).unwrap();
    sim.push(Command::Stockpile { a: at, b: at.offset(2, 0), zone: None });
    sim.step();
    sim.world.put_lot(Lot::new(stone, 30), at);
    sim.world.put_lot(Lot::new(wood, 12), at.offset(1, 0));
    sim.world.place_item(stone, at.offset(0, 4), 9);
    (sim.world.zones.list[0].id, at)
}

/// A UI mod that prints every row of view.stock as "thing stored/loose/stores".
const PROBE: &str = r#"
ui.define("probe:stock", function(view)
    local out = {}
    for _, r in view.stock() do
        table.insert(out, { kind = "text", string.format("%s %d/%d/%d", r.thing, r.stored, r.loose, r.stores) })
    end
    return { kind = "col", table.unpack(out) }
end)
ui.mount("left", "probe:stock")
"#;

#[test]
fn view_stock_and_the_sheet_match_the_ledger_for_every_thing() {
    let dir = scratch_mods("stores-sheet", &[("probe", "", &[("ui/probe.luau", PROBE)])]);
    let mut sim = sim_at(&dir);
    stockpile(&mut sim);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Input::default());
    let text = ui.snapshot();
    let w = &sim.world;
    let mut stored_all = 0;
    let mut loose_all = 0;
    for d in 0..w.defs.things.len() as rim_sim::defs::DefId {
        let total = w.stock.on_map(d);
        if total == 0 {
            continue;
        }
        let stored = w.stock.stored(d);
        stored_all += stored;
        loose_all += total - stored;
        // Stores by the ledger's own reading: this test has one stockpile.
        let stores = u32::from(stored > 0);
        let want = format!("{} {}/{}/{}", w.defs.thing(d).id, stored, total - stored, stores);
        assert!(text.contains(&want), "{want} in:\n{text}");
    }
    // The sheet's totals are the same sums.
    ui.open_window("core:stores");
    frame(&mut ui, &sim, &cv, Input { time: 1.0, ..Default::default() });
    let sheet = ui.snapshot();
    assert!(sheet.contains(&format!("{stored_all} stored")), "{sheet}");
    assert!(sheet.contains(&format!("{loose_all} loose")), "{sheet}");
    assert!(sheet.contains("stone blocks") && sheet.contains("7 days"), "{sheet}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_card_shows_a_store_once_the_cursor_rests_on_it() {
    let mut sim = sim_at(&mods());
    let (_, at) = stockpile(&mut sim);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.hover_cell = Some(at);
    cv.time = 10.0;
    frame(&mut ui, &sim, &cv, Input { time: 10.0, ..Default::default() });
    assert!(ui.find("core:store.card").is_none(), "not at once");
    cv.time = 10.1;
    frame(&mut ui, &sim, &cv, Input { time: 10.1, ..Default::default() });
    assert!(ui.find("core:store.card").is_none(), "not before it has rested");
    cv.time = 10.5;
    frame(&mut ui, &sim, &cv, Input { time: 10.5, ..Default::default() });
    assert!(ui.find("core:store.card").is_some(), "{}", ui.snapshot());
    let text = ui.snapshot();
    assert!(text.contains("Stockpile 1") && text.contains("Normal") && text.contains("2 of 3 cells"), "{text}");
    assert!(ui.find("core:store.card.tokens").is_some(), "its tokens");
    // Off the store, the card goes.
    cv.hover_cell = Some(sim.world.colony_center().unwrap());
    cv.time = 11.0;
    frame(&mut ui, &sim, &cv, Input { time: 11.0, ..Default::default() });
    assert!(ui.find("core:store.card").is_none());
}

const CRATE: &str = r##"
[[thing]]
id = "crate"
label = "crate"
color = "#b98a55"
category = "building"
blocks = true
store = { slots = 4 }
"##;

#[test]
fn the_card_costs_little_over_two_hundred_stores() {
    let dir = scratch_mods("stores-card-200", &[("probe", "", &[("defs/crate.toml", CRATE)])]);
    let mut sim = sim_at(&dir);
    let def = sim.world.defs.thing_id("probe:crate").unwrap();
    let berries = sim.world.defs.thing_id("berries").unwrap();
    let c = sim.world.colony_center().unwrap();
    let mut placed = Vec::new();
    for r in 3..120i32 {
        for (dx, dy) in [(r, 0), (-r, 0), (0, r), (0, -r), (r, r), (-r, -r), (r, -r), (-r, r)] {
            let p = c.offset(dx, dy);
            if placed.len() < 200
                && sim.world.map.passable(p)
                && sim.world.map.fixture_at(p).is_none()
                && sim.world.map.item_at(p).is_none()
            {
                if let Some(e) = sim.world.spawn_fixture_of(def, p, false, None) {
                    sim.world.put_in_store(e, Lot::new(berries, 7));
                    placed.push(p);
                }
            }
        }
    }
    assert_eq!(placed.len(), 200);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    // Rebuild frames (shift flips, so the client hash moves), on a crate
    // with its card up and on open ground without one.
    let time = |ui: &mut rim_ui::Ui, cv: &mut rim_ui::view::ClientView, at| {
        cv.hover_cell = Some(at);
        for i in 0..3 {
            cv.time = 20.0 + i as f64;
            frame(ui, &sim, cv, Input { time: cv.time, ..Default::default() });
        }
        // The fastest of many rebuilds, as the other budget tests take it:
        // a mean under a loaded machine measures the machine.
        let mut best = f64::MAX;
        for i in 0..40 {
            cv.shift = i % 2 == 0;
            let t0 = std::time::Instant::now();
            frame(ui, &sim, cv, Input { time: cv.time, ..Default::default() });
            best = best.min(t0.elapsed().as_secs_f64() * 1e3);
        }
        best
    };
    let with = time(&mut ui, &mut cv, placed[0]);
    assert!(ui.find("core:store.card").is_some(), "the card is up");
    let without = time(&mut ui, &mut cv, c);
    eprintln!("fastest UI rebuild frame, 200 containers: {with:.3} ms with a store card, {without:.3} ms without");
    // DESIGN.md §11's budget for a frame that rebuilds trees is 2 ms, the
    // one engine.rs's budget test holds rebuilds to, with its slack for
    // shared CI runners (3x, and 6x on Windows). The difference with and
    // without the card is printed, not asserted: two noisy timings
    // subtracted measure the machine (0.99 against 0.46 ms at load 65).
    let slack = match (std::env::var_os("CI").is_some(), cfg!(windows)) {
        (false, _) => 1.0,
        (true, false) => 3.0,
        (true, true) => 6.0,
    };
    // Timed only alone, in CI's budget step (DESIGN.md §8a).
    if timing_budgets() {
        assert!(with < 2.0 * slack, "rebuild with the card up: {with:.3} ms (budget {:.1} ms)", 2.0 * slack);
    }
    let _ = std::fs::remove_dir_all(dir);
}
