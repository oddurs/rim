//! The item token (DESIGN.md §4f): a thing's world look, count, notch,
//! condition and state, drawn by one component everywhere.

mod common;
use common::*;
use rim_ui::paint::Draw;

const PROBE: &str = r#"
local kit = require("@core/ui/kit")
ui.define("probe:items", function(view)
    return kit.row({ id = "probe:items", gap = "s" }, {
        kit.item({ id = "probe:wood", thing = "core:wood", count = 140 }),
        kit.item({ id = "probe:full", thing = "core:stone", count = 75, limit = 75 }),
        kit.item({ id = "probe:worn", thing = "core:raw_meat", count = 1, hp = 0.25 }),
        kit.item({ id = "probe:small", thing = "core:wood", count = 140, size = "s" }),
        kit.item({ id = "probe:empty", state = "empty" }),
        kit.item({ id = "probe:leaving", thing = "core:berries", count = 3, state = "leaving" }),
        kit.label(table.concat({ kit.item_count(7), kit.item_count(999), kit.item_count(1020), kit.item_count(12480), kit.item_count(1) }, "|"), { id = "probe:counts" }),
    })
end)
ui.mount("windows", "probe:items")
"#;

const GRID: &str = r#"
local kit = require("@core/ui/kit")
local things = { "core:wood", "core:stone", "core:berries", "core:raw_meat" }
ui.define("probe:grid", function(view)
    return ui.grid({ id = "probe:grid", rows = 10, cols = 20, cell_w = 32, cell_h = 32, gap = 4,
        cell = function(r, c)
            local i = (r * 20 + c)
            return { token = kit.item_token({ thing = things[i % 4 + 1], count = i * 7, limit = 75, hp = (i % 9 == 0) and 0.5 or nil }) }
        end })
end)
ui.mount("windows", "probe:grid")
"#;

fn inside(r: [f32; 4], o: [f32; 4]) -> bool {
    r[0] >= o[0] - 0.01 && r[1] >= o[1] - 0.01 && r[0] + r[2] <= o[0] + o[2] + 0.01 && r[1] + r[3] <= o[1] + o[3] + 0.01
}

fn near(a: [f32; 4], b: [f32; 4]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 0.02)
}

/// Rects drawn inside `area`, with their colours.
fn rects_in(draw: &[Draw], area: [f32; 4]) -> Vec<([f32; 4], [f32; 4])> {
    draw.iter()
        .filter_map(|d| match d {
            Draw::Rect { rect, color, .. } if inside(*rect, area) => Some((*rect, *color)),
            _ => None,
        })
        .collect()
}

fn glyphs_in(draw: &[Draw], area: [f32; 4]) -> usize {
    draw.iter().filter(|d| matches!(d, Draw::Glyphs { quads, .. } if quads.iter().all(|q| inside(q.dst, area)))).count()
}

fn rgb(c: [u8; 3]) -> [f32; 4] {
    [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, 1.0]
}

#[test]
fn a_token_draws_the_things_own_look_its_count_and_its_state() {
    let dir = scratch_mods("tokens", &[("probe", "", &[("ui/probe.luau", PROBE)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    let out = frame(&mut ui, &sim, &cv, Default::default());
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    let snap = ui.snapshot();
    assert!(snap.contains("token #probe:wood \"140\" Plain"), "{snap}");
    assert!(snap.contains("token #probe:full \"75\" Plain full"), "{snap}");
    assert!(snap.contains("token #probe:worn \"\" Plain"), "a count of one is never drawn: {snap}");
    assert!(snap.contains("token #probe:small \"\" Plain"), "a small token draws no count: {snap}");
    assert!(snap.contains("token #probe:empty \"\" Empty"), "{snap}");
    assert!(snap.contains("token #probe:leaving \"3\" Leaving"), "{snap}");
    assert!(snap.contains("\"7|999|1.0k|12k|\""), "counts read exact to 999, then 1.0k and 12k: {snap}");

    let defs = &sim.world.defs;
    let colour = |id: &str| rgb(defs.thing(defs.thing_id(id).unwrap()).rgb);
    let wood = ui.find("probe:wood").unwrap();
    assert_eq!((wood[2], wood[3]), (32.0, 32.0), "a medium token is 32 px");
    let drawn = rects_in(&out.draw, wood);
    assert!(drawn.iter().any(|(_, c)| near(*c, colour("core:wood"))), "the wood's own fill: {drawn:?}");
    assert!(glyphs_in(&out.draw, wood) >= 2, "a count, over its shadow");

    let small = ui.find("probe:small").unwrap();
    assert_eq!((small[2], small[3]), (20.0, 20.0));
    assert_eq!(glyphs_in(&out.draw, small), 0);

    // Full: a notch in the top right corner, in the text colour.
    let full = ui.find("probe:full").unwrap();
    let notch = rects_in(&out.draw, full).into_iter().filter(|(r, _)| r[1] < full[1] + 3.0 && r[0] > full[0] + 20.0);
    assert!(notch.count() >= 1, "a notch on the full stack");

    // Worn: a bar a quarter full along the bottom.
    let worn = ui.find("probe:worn").unwrap();
    let bottom: Vec<_> =
        rects_in(&out.draw, worn).into_iter().filter(|(r, _)| r[1] > worn[1] + worn[3] - 3.0).collect();
    assert!(bottom.iter().any(|(r, _)| r[2] > 0.0 && r[2] < worn[2] * 0.3), "a quarter-long bar: {bottom:?}");

    // Empty: dashes and nothing else.
    let empty = ui.find("probe:empty").unwrap();
    let dashes = rects_in(&out.draw, empty);
    assert!(dashes.len() >= 8 && dashes.iter().all(|(r, _)| r[2] <= 3.01 || r[3] <= 3.01), "{dashes:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_grid_of_tokens_is_one_node_and_draws_every_look() {
    let dir = scratch_mods("tokengrid", &[("probe", "", &[("ui/probe.luau", GRID)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    let out = frame(&mut ui, &sim, &cv, Default::default());
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    let snap = ui.snapshot();
    let tree = snap.split("== probe:grid").nth(1).unwrap().split("== ").next().unwrap();
    assert_eq!(tree.trim().lines().count(), 1, "one node: {tree}");
    let grid = ui.find("probe:grid").unwrap();
    let stone = rgb(sim.world.defs.thing(sim.world.defs.thing_id("stone").unwrap()).rgb);
    let fills = rects_in(&out.draw, grid).into_iter().filter(|(_, c)| near(*c, stone)).count();
    assert!(fills >= 50, "every stone token's look: {fills}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_look_for_a_thing_that_does_not_exist_is_an_error_not_a_blank() {
    let bad = r#"
local kit = require("@core/ui/kit")
ui.define("probe:bad", function(view) return kit.item({ thing = "core:unobtainium", count = 2 }) end)
ui.mount("windows", "probe:bad")
"#;
    let dir = scratch_mods("tokenbad", &[("probe", "", &[("ui/probe.luau", bad)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    assert!(ui.snapshot().contains("no thing 'core:unobtainium'"), "{}", ui.snapshot());
    let _ = std::fs::remove_dir_all(&dir);
}

/// 200 tokens on screen, rebuilt at the UI's rate: what a full contents
/// grid costs. Printed for the item's notes; held to the UI's 1 ms budget
/// with the same CI slack the frame budget tests use.
#[test]
fn two_hundred_tokens_fit_the_frame_budget() {
    let dir = scratch_mods("tokenbudget", &[("probe", "", &[("ui/probe.luau", GRID)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    for _ in 0..30 {
        frame(&mut ui, &sim, &cv, Default::default());
    }
    let (mut all, mut builds) = (Vec::new(), Vec::new());
    for i in 0..120 {
        let mouse = (400.0 + i as f32, 300.0);
        let before = ui.builds;
        let t = std::time::Instant::now();
        frame(&mut ui, &sim, &cv, rim_ui::Input { mouse, time: 10.0 + i as f64 / 60.0, ..Default::default() });
        let ms = t.elapsed().as_secs_f64() * 1e3;
        all.push(ms);
        if ui.builds > before {
            builds.push(ms);
        }
    }
    let median = |v: &mut Vec<f64>| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[v.len() / 2]
    };
    let (m, b) = (median(&mut all), median(&mut builds));
    let fastest = builds[0];
    println!("200 tokens: median frame {m:.3} ms, rebuild frames median {b:.3} ms, fastest {fastest:.3} ms");
    // The frame budget tests' own bars: a median frame under 1 ms, and the
    // fastest rebuild (the machine's cost, not the contention's) under 2.
    let slack = if std::env::var_os("CI").is_some() {
        if cfg!(windows) {
            6.0
        } else {
            3.0
        }
    } else {
        1.5
    };
    // Timed only alone, in CI's budget step (DESIGN.md §8a).
    if timing_budgets() {
        assert!(m < 1.0 * slack, "median frame {m:.3} ms");
        assert!(fastest < 2.0 * slack, "fastest rebuild {fastest:.3} ms");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
