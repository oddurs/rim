//! The UI engine, headless: layout, routing, the VM boundary, mod operations,
//! tokens, anchored placement, hot reload and the frame budget.

mod common;
use common::*;
use rim_ui::paint::Draw;
use rim_ui::view::UiAction;
use rim_ui::Input;

#[test]
fn shell_docks_regions_to_the_edges() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.hover_cell = sim.world.colony_center();
    frame(&mut ui, &sim, &cv, Default::default());
    let top = ui.find("core:topbar").expect("top bar laid out");
    let bottom = ui.find("core:toolbar.buttons").expect("toolbar laid out");
    assert_eq!(top[1], 0.0, "top bar sits at the top");
    assert!((top[2] - 1600.0).abs() < 1.0, "top bar spans the width: {top:?}");
    assert!((bottom[1] + bottom[3] - 960.0).abs() < 1.0, "toolbar sits at the bottom: {bottom:?}");
    // Docked panels sit against their edges, not merely somewhere inside.
    let near = |a: f32, b: f32| (a - b).abs() <= 12.0;
    let inspector = ui.find("core:inspector").unwrap();
    assert!(inspector[0] < 12.0, "inspector docks left: {inspector:?}");
    assert!(near(inspector[1] + inspector[3], bottom[1]), "inspector sits on the toolbar: {inspector:?} vs {bottom:?}");
    let messages = ui.find("core:messages").unwrap();
    assert!(near(messages[1], top[1] + top[3]), "messages start just under the top bar: {messages:?}");
    let hover = ui.find("core:hover").unwrap();
    assert!(near(hover[0] + hover[2], 1600.0), "hover readout docks right: {hover:?}");
    assert!(near(hover[1] + hover[3], bottom[1]), "hover readout sits on the toolbar: {hover:?}");
    // Grown boxes take their share: the top bar's spacer pushes the help text right.
    let right = ui.find("core:topbar.right").unwrap();
    assert!(right[0] > 600.0, "the top bar's spacer should grow: {right:?}");
}

#[test]
fn layout_is_cached_until_the_tree_or_viewport_changes() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let n = ui.info.layouts;
    for _ in 0..5 {
        frame(&mut ui, &sim, &cv, Default::default());
    }
    assert_eq!(ui.info.layouts, n, "unchanged frames re-laid out the shell");
    cv.screen = (1280.0, 800.0);
    frame(&mut ui, &sim, &cv, Default::default());
    assert_eq!(ui.info.layouts, n + 1, "a new viewport lays out once");
    let top = ui.find("core:topbar").unwrap();
    assert!((top[2] - 1280.0).abs() < 1.0, "and the shell reflows to it");
}

#[test]
fn clicks_on_panels_never_reach_the_world() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let chop = ui.find("core:toolbar.designate:chop").unwrap();
    let actions = click(&mut ui, &sim, &mut cv, centre(chop));
    assert_eq!(actions, vec![UiAction::Tool("designate:chop".into())]);

    // A panel's background swallows the click but does nothing.
    let panel = ui.find("core:inspector").unwrap();
    let at = (panel[0] + 4.0, panel[1] + 4.0);
    let out = frame(&mut ui, &sim, &cv, Input { mouse: at, left_pressed: true, ..Default::default() });
    assert!(out.mouse_over_ui && out.captured_left, "panel background should capture the click");

    // Open ground in the middle belongs to the world.
    let out = frame(&mut ui, &sim, &cv, Input { mouse: (800.0, 480.0), left_pressed: true, ..Default::default() });
    assert!(!out.mouse_over_ui && !out.captured_left, "the world should get this click");
}

#[test]
fn colonist_bar_selects_and_focuses() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.selected = None;
    frame(&mut ui, &sim, &cv, Default::default());
    let founder = sim.world.colonists().next().unwrap();
    let name = sim.world.ecs.get::<&rim_sim::world::Pawn>(founder).unwrap().name.clone();
    let b = ui.find(&format!("core:colonists.{name}")).expect("colonist button");
    let actions = click(&mut ui, &sim, &mut cv, centre(b));
    assert_eq!(actions, vec![UiAction::Select(Some(founder)), UiAction::Focus(founder)]);
}

#[test]
fn ui_cannot_change_the_simulation() {
    // A hostile UI mod: it tries to reach the engine, overwrite the API and
    // rewrite what view returns. None of it may touch the world.
    let dir = scratch_mods(
        "hostile",
        &[(
            "sneaky",
            "",
            &[(
                "ui/evil.luau",
                r#"
local ok1 = pcall(function() ui.define = nil end)
local ok2 = pcall(function() view.tick = function() return 0 end end)
local ok3 = pcall(function() act.tool = nil end)
local ok4 = rim == nil and os == nil and io == nil
ui.define("sneaky:report", function(view)
  return ui.text { string.format("frozen=%s noengine=%s", tostring(not (ok1 or ok2 or ok3)), tostring(ok4)) }
end)
ui.mount("top", "sneaky:report", { order = 99 })
"#,
            )],
        )],
    );
    let mut with_ui = sim_at(&dir);
    let mut without = sim_at(&dir);
    let mut ui = ui_for(&with_ui);
    for _ in 0..600 {
        with_ui.step();
        without.step();
        let cv = client(&with_ui);
        let out = frame(
            &mut ui,
            &with_ui,
            &cv,
            Input { mouse: (800.0, 480.0), left_pressed: true, left_released: true, ..Default::default() },
        );
        let _ = out.actions; // actions go to the client, never to the sim
    }
    assert_eq!(with_ui.world.state_hash(), without.world.state_hash(), "building the UI changed the simulation");
    let snap = ui.snapshot();
    assert!(snap.contains("frozen=true noengine=true"), "the sandbox let a UI script through:\n{snap}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mods_extend_replace_wrap_and_remove() {
    let dir = scratch_mods(
        "ops",
        &[
            (
                "alpha",
                "",
                &[(
                    "ui/alpha.luau",
                    r#"
ui.extend("core:topbar.right", ui.text { "alpha was here", id = "alpha:note" })
ui.replace("core:hover", function(view) return ui.text { "alpha hover", id = "alpha:hover" } end)
ui.wrap("core:clock", function(inner, view) return ui.row { id = "alpha:wrapped", inner } end)
ui.remove("core:messages")
ui.extend("core:nothing_here", ui.text { "lost" })
"#,
                )],
            ),
            (
                "beta",
                "load_after = [\"alpha\"]",
                &[("ui/beta.luau", r#"ui.replace("core:hover", function(view) return ui.text { "beta hover" } end)"#)],
            ),
        ],
    );
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.hover_cell = sim.world.colony_center();
    frame(&mut ui, &sim, &cv, Default::default());
    let snap = ui.snapshot();
    assert!(snap.contains("\"alpha was here\""), "extend didn't add to the top bar:\n{snap}");
    assert!(
        snap.contains("\"beta hover\"") && !snap.contains("\"alpha hover\""),
        "load order should pick beta's replacement:\n{snap}"
    );
    assert!(snap.contains("#alpha:wrapped"), "wrap didn't wrap the clock:\n{snap}");
    assert!(!snap.contains("== core:messages\ncol"), "remove didn't remove messages:\n{snap}");
    let warnings = ui.warnings();
    assert!(
        warnings.iter().any(|w| w.contains("UI conflict") && w.contains("'alpha'") && w.contains("'beta'")),
        "{warnings:#?}"
    );
    assert!(warnings.iter().any(|w| w.contains("core:nothing_here")), "unknown ids should be reported: {warnings:#?}");
    // Nodes remember who made them (devtools shows this).
    let top = ui.last_trees.iter().find(|(id, _)| id == "core:topbar").unwrap();
    fn find<'a>(n: &'a rim_ui::node::Node, id: &str) -> Option<&'a rim_ui::node::Node> {
        if n.id.as_deref() == Some(id) {
            return Some(n);
        }
        n.children.iter().find_map(|c| find(c, id))
    }
    assert_eq!(&*find(&top.1, "alpha:note").unwrap().owner, "alpha");
    assert_eq!(&*find(&top.1, "core:colonists").unwrap().owner, "core");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_broken_component_shows_an_error_and_the_rest_keeps_working() {
    let dir = scratch_mods(
        "broken",
        &[(
            "oops",
            "",
            &[(
                "ui/oops.luau",
                r#"
ui.define("oops:bad", function(view) error("kaboom") end)
ui.mount("top", "oops:bad", { order = 5 })
ui.define("oops:typo", function(view) return ui.text { "x", color = "not_a_token" } end)
ui.mount("top", "oops:typo", { order = 6 })
"#,
            )],
        )],
    );
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let snap = ui.snapshot();
    assert!(snap.contains("kaboom"), "the error should be shown in place:\n{snap}");
    assert!(snap.contains("unknown color token 'not_a_token'"), "bad tokens should be named:\n{snap}");
    assert!(ui.find("core:toolbar.buttons").is_some(), "the rest of the UI must still build");
    assert!(ui.warnings().iter().any(|w| w.contains("oops") && w.contains("kaboom")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_endless_loop_in_a_component_is_stopped() {
    let dir = scratch_mods(
        "endless",
        &[(
            "spin",
            "",
            &[(
                "ui/spin.luau",
                r#"
ui.define("spin:forever", function(view) while true do end end)
ui.mount("top", "spin:forever", { order = 5 })
"#,
            )],
        )],
    );
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    let t = std::time::Instant::now();
    frame(&mut ui, &sim, &cv, Default::default());
    assert!(t.elapsed() < std::time::Duration::from_secs(5), "the frame came back");
    let snap = ui.snapshot();
    assert!(snap.contains("endless loop"), "the component shows why it stopped:\n{snap}");
    assert!(ui.find("core:toolbar.buttons").is_some(), "the rest of the UI still builds");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_scroll_area_stops_exactly_at_its_last_row() {
    let dir = scratch_mods(
        "scroll",
        &[(
            "lister",
            "",
            &[(
                "ui/list.luau",
                r#"
ui.define("lister:list", function(view)
    local rows = {}
    for i = 1, 30 do
        table.insert(rows, ui.text({ "row " .. i, id = "lister:row" .. i }))
    end
    return ui.col({ id = "lister:panel", bg = "surface", pad = 0,
        { kind = "scroll", id = "lister:scroll", h = 120, gap = 3, pad = 9, table.unpack(rows) } })
end)
ui.mount("windows", "lister:list")
"#,
            )],
        )],
    );
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let area = ui.find("lister:scroll").expect("the scroll area is laid out");
    let at = centre(area);
    // Scroll far past the end: it must stop where the last row sits just
    // above the bottom padding.
    for _ in 0..200 {
        frame(&mut ui, &sim, &cv, Input { mouse: at, wheel: -5.0, ..Default::default() });
    }
    frame(&mut ui, &sim, &cv, Input { mouse: at, ..Default::default() });
    // Rects from `find` are unscrolled: the furthest scroll is where the last
    // row's bottom, plus the bottom padding, meets the area's bottom.
    let last = ui.find("lister:row30").expect("last row");
    let pad = 9.0 * ui.theme.scale;
    let want = (last[1] + last[3] + pad) - (area[1] + area[3]);
    let got = ui.scroll_offset("lister:scroll").expect("scroll state");
    assert!(want > 100.0, "the list overflows its area ({want} px)");
    assert!((got - want).abs() <= 1.0, "scrolled to {got} px, the end is at {want} px");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn theme_tokens_can_be_overridden_and_conflicts_are_reported() {
    let dir = scratch_mods(
        "tokens",
        &[
            ("red", "", &[("ui/theme.toml", "[color]\naccent = \"#ff0000\"\n")]),
            (
                "green",
                "load_after = [\"red\"]",
                &[("ui/theme.toml", "[color]\naccent = \"#00ff00\"\n[space]\nm = 10\n")],
            ),
        ],
    );
    let sim = sim_at(&dir);
    let ui = ui_for(&sim);
    assert_eq!(ui.theme.color["accent"], [0.0, 1.0, 0.0, 1.0], "last override wins");
    assert_eq!(ui.theme.space["m"], 10.0);
    assert_eq!(ui.theme.set_by["color.accent"], "green");
    assert!(
        ui.warnings().iter().any(|w| w.contains("theme conflict") && w.contains("color.accent")),
        "{:#?}",
        ui.warnings()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ui_scale_scales_every_size() {
    let sim = sim_at(&mods());
    let mut small = ui_for(&sim);
    let mut big = rim_ui::Ui::new(rim_ui::mods_of(&sim), 2.0, 1.0).unwrap();
    let cv = client(&sim);
    let mut cv2 = cv.clone();
    cv2.screen = (3200.0, 1920.0);
    cv2.scale = 2.0;
    frame(&mut small, &sim, &cv, Default::default());
    frame(&mut big, &sim, &cv2, Default::default());
    let a = small.find("core:topbar").unwrap()[3];
    let b = big.find("core:topbar").unwrap()[3];
    assert!((b / a - 2.0).abs() < 0.15, "top bar height {a} at 1x vs {b} at 2x");
}

#[test]
fn anchored_labels_never_overlap() {
    let mut sim = sim_at(&mods());
    let human = sim.world.defs.creature_id("human").unwrap();
    let c = sim.world.colony_center().unwrap();
    // A crowd of ten colonists on two cells.
    for i in 0..10 {
        let p = if sim.world.map.passable(c.offset(i % 2, 0)) { c.offset(i % 2, 0) } else { c };
        sim.world.spawn_pawn(human, rim_sim::world::Faction::Player, p, None);
    }
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    let out = frame(&mut ui, &sim, &cv, Default::default());
    let _ = out;
    // Collect the rects of anchored labels from the draw list: glyph runs
    // drawn in the anchored layer are the first ones, before the shell.
    let rects: Vec<[f32; 4]> = ui_rects_of_labels(&mut ui, &sim, &cv);
    assert!(rects.len() >= 10, "expected a label per colonist, got {}", rects.len());
    for (i, a) in rects.iter().enumerate() {
        for b in &rects[i + 1..] {
            let overlap = a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3];
            assert!(!overlap, "labels overlap: {a:?} and {b:?}");
        }
    }
}

fn ui_rects_of_labels(ui: &mut rim_ui::Ui, sim: &rim_sim::Sim, cv: &rim_ui::view::ClientView) -> Vec<[f32; 4]> {
    let out = frame(ui, sim, cv, Default::default());
    let mut rects = Vec::new();
    for d in &out.draw {
        if let Draw::Glyphs { quads, .. } = d {
            // Stop at the first glyph run below the top bar's band: anchored
            // labels come first in the draw list.
            let x0 = quads.iter().map(|q| q.dst[0]).fold(f32::MAX, f32::min);
            let y0 = quads.iter().map(|q| q.dst[1]).fold(f32::MAX, f32::min);
            let x1 = quads.iter().map(|q| q.dst[0] + q.dst[2]).fold(f32::MIN, f32::max);
            let y1 = quads.iter().map(|q| q.dst[1] + q.dst[3]).fold(f32::MIN, f32::max);
            rects.push([x0, y0, x1 - x0, y1 - y0]);
        } else if let Draw::Rect { rect, .. } = d {
            if rect[1] == 0.0 {
                break; // the top bar: the shell has started
            }
        }
    }
    rects
}

#[test]
fn tab_moves_focus_and_enter_clicks() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    // Tab with nothing focused is the game's (next colonist).
    let out = frame(&mut ui, &sim, &cv, Input { tab: true, ..Default::default() });
    assert!(!out.captured_keys, "tab with no UI focus should go to the game");
    // Click a toolbar button: it takes focus; Tab moves on; Enter activates.
    let select = ui.find("core:toolbar.select").unwrap();
    let mut cv2 = cv.clone();
    click(&mut ui, &sim, &mut cv2, centre(select));
    let out = frame(&mut ui, &sim, &cv, Input { tab: true, ..Default::default() });
    assert!(out.captured_keys, "tab should move focus between UI controls");
    let out = frame(&mut ui, &sim, &cv, Input { enter: true, ..Default::default() });
    assert!(!out.actions.is_empty(), "enter should activate the focused button");
    ui.blur();
    let out = frame(&mut ui, &sim, &cv, Input { tab: true, ..Default::default() });
    assert!(!out.captured_keys, "after blur, tab goes back to the game");
}

#[test]
fn hot_reload_swaps_the_ui_and_keeps_the_last_good_one_on_error() {
    let dir = scratch_mods("reload", &[]);
    let mut sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let before = sim.world.state_hash();

    let hud = dir.join("core/ui/hud.luau");
    let src = std::fs::read_to_string(&hud).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&hud, src.replace("\"Wealth \"", "\"Riches \"")).unwrap();
    assert!(ui.check_reload(10.0), "a changed UI script should reload");
    frame(&mut ui, &sim, &cv, Default::default());
    assert!(ui.snapshot().contains("Riches"), "the new script should be running");

    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&hud, "this is not luau (").unwrap();
    assert!(!ui.check_reload(20.0), "a broken script must not replace the running UI");
    assert!(ui.reload_error.is_some());
    frame(&mut ui, &sim, &cv, Default::default());
    assert!(ui.snapshot().contains("Riches"), "the last good UI keeps running");

    sim.step();
    let mut fresh = sim_at(&dir);
    fresh.step();
    assert_eq!(sim.world.state_hash(), fresh.world.state_hash(), "reloading the UI touched the simulation");
    let _ = before;
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn whole_ui_fits_the_frame_budget_with_30_colonists() {
    let mut sim = sim_at(&mods());
    let human = sim.world.defs.creature_id("human").unwrap();
    let c = sim.world.colony_center().unwrap();
    for i in 0..29 {
        let p = c.offset(i % 6 - 3, i / 6 - 2);
        if sim.world.map.passable(p) {
            sim.world.spawn_pawn(human, rim_sim::world::Faction::Player, p, None);
        }
    }
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.show_profiler = true;
    cv.hover_cell = Some(c);
    for _ in 0..30 {
        frame(&mut ui, &sim, &cv, Default::default());
    }
    // Two seconds of frames at 60 fps with the mouse moving. Wall-clock
    // numbers on a busy machine are noisy, so compare medians, and assert
    // the work the design guarantees (rebuild rate) exactly.
    let n = 120;
    let builds_before = ui.builds;
    let mut plain = Vec::new();
    let mut rebuilds = Vec::new();
    for i in 0..n {
        cv.mouse = (400.0 + i as f32, 300.0);
        let before = ui.builds;
        let f = std::time::Instant::now();
        frame(&mut ui, &sim, &cv, Input { mouse: cv.mouse, time: 10.0 + i as f64 / 60.0, ..Default::default() });
        let ms = f.elapsed().as_secs_f64() * 1e3;
        if ui.builds > before {
            rebuilds.push(ms)
        } else {
            plain.push(ms)
        }
    }
    let median = |v: &mut Vec<f64>| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[v.len() / 2]
    };
    let mut all: Vec<f64> = plain.iter().chain(&rebuilds).copied().collect();
    let (m_all, m_plain, m_build) = (median(&mut all), median(&mut plain), median(&mut rebuilds));
    let built = ui.builds - builds_before;
    println!(
        "whole UI, {} nodes: median frame {m_all:.3} ms (paint-only {m_plain:.3} ms, rebuild {m_build:.3} ms); {built} rebuilds in {n} frames",
        ui.info.nodes
    );
    println!("  luau time by mod: {:?}", ui.vm.mod_time);
    assert!((20..=45).contains(&built), "expected ~20 Hz rebuilds (40 in 2 s), got {built}");
    // Shared CI runners are 2-3x slower than a laptop and noisy with it: the
    // same binary measured 0.9 ms locally and 2.0-2.4 ms on CI. The budgets
    // are for a player's machine, so CI gets slack that still catches a real
    // regression, and the rebuild rate above is asserted exactly everywhere.
    let slack = if std::env::var_os("CI").is_some() { 3.0 } else { 1.0 };
    assert!(m_all < 1.0 * slack, "median frame {m_all:.3} ms (budget {:.1} ms)", 1.0 * slack);
    assert!(m_build < 2.0 * slack, "median rebuild frame {m_build:.3} ms (budget {:.1} ms)", 2.0 * slack);
}

#[test]
fn wildlife_plus_extends_the_top_bar() {
    // The shipped example plugin changes core's HUD through ui.extend.
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let wild = sim
        .world
        .pawns
        .iter()
        .filter(|&&e| {
            sim.world.ecs.get::<&rim_sim::world::Pawn>(e).is_ok_and(|p| p.faction == rim_sim::world::Faction::Wild)
        })
        .count();
    assert!(ui.snapshot().contains(&format!("\"wildlife {wild}\"")), "{}", ui.snapshot());
    let counter = ui.find("wildlife_plus:counter").expect("counter laid out");
    let top = ui.find("core:topbar").unwrap();
    assert!(counter[1] >= top[1] && counter[1] + counter[3] <= top[1] + top[3], "the counter sits in the top bar");
    let top_tree = &ui.last_trees.iter().find(|(id, _)| id == "core:topbar").unwrap().1;
    fn owner_of(n: &rim_ui::node::Node, id: &str) -> Option<String> {
        if n.id.as_deref() == Some(id) {
            return Some(n.owner.to_string());
        }
        n.children.iter().find_map(|c| owner_of(c, id))
    }
    assert_eq!(owner_of(top_tree, "wildlife_plus:counter").as_deref(), Some("wildlife_plus"));
}

/// Every Luau sample in the UI modding guide loads and builds cleanly, so
/// the guide can't drift from what the engine accepts.
#[test]
fn guide_samples_run() {
    // Tolerate CRLF checkouts (a player's own files may have them).
    let guide = std::fs::read_to_string(mods().join("../docs/modding/ui.md")).unwrap().replace("\r\n", "\n");
    let blocks: Vec<&str> = guide.split("```lua\n").skip(1).map(|b| b.split("```").next().unwrap()).collect();
    assert!(blocks.len() >= 4, "found {} samples", blocks.len());
    for (i, block) in blocks.iter().enumerate() {
        let file = format!("ui/sample{i}.luau");
        let dir = scratch_mods(&format!("guide{i}"), &[("my_mod", "", &[(file.as_str(), block)])]);
        let sim = sim_at(&dir);
        let mut ui = ui_for(&sim);
        let mut cv = client(&sim);
        cv.hover_cell = sim.world.colony_center();
        frame(&mut ui, &sim, &cv, Default::default());
        let problems: Vec<String> = ui.warnings();
        assert!(problems.is_empty(), "guide sample {i} failed:\n{block}\n{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn a_theme_can_name_a_font_and_a_missing_one_falls_back() {
    // Ask for whatever family the system UI font resolved to: it exists here.
    let sim = sim_at(&mods());
    let system = ui_for(&sim).text.info.family.clone();
    let dir =
        scratch_mods("font", &[("typeface", "", &[("ui/theme.toml", &format!("[font]\nfamily = \"{system}\"\n"))])]);
    let ui = ui_for(&sim_at(&dir));
    assert_eq!(ui.text.info.family, system);
    assert!(ui.text.info.source.starts_with("family"), "the theme's family should be used: {}", ui.text.info.source);
    assert!(ui.warnings().is_empty(), "{:#?}", ui.warnings());
    let _ = std::fs::remove_dir_all(&dir);

    let dir =
        scratch_mods("nofont", &[("typeface", "", &[("ui/theme.toml", "[font]\nfamily = \"No Such Font 9000\"\n")])]);
    let ui = ui_for(&sim_at(&dir));
    assert_eq!(ui.text.info.family, system, "falls back to the system UI font");
    assert!(
        ui.warnings().iter().any(|w| w.contains("typeface") && w.contains("No Such Font 9000")),
        "{:#?}",
        ui.warnings()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The material row (0215): with a stuff buildable selected, the toolbar
/// grows a row of materials, what you have none of says so, and the rest
/// of the toolbar is still there. Headless, so it runs on every platform.
#[test]
fn the_material_row_renders_beside_the_toolbar() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    for t in &mut cv.tools {
        t.active = t.key == "build:wall";
    }
    let mat = |id: &str, label: &str, have: u32, active: bool| rim_ui::view::StuffView {
        id: id.into(),
        label: label.into(),
        color: [200, 180, 160],
        have,
        active,
        hp: if active { 140 } else { 320 },
        work: if active { 144 } else { 324 },
    };
    cv.stuff = vec![mat("wood", "wood", 12, true), mat("stone", "stone blocks", 0, false)];
    frame(&mut ui, &sim, &cv, Default::default());
    let snap = ui.snapshot();
    assert!(ui.find("core:toolbar.buttons").is_some(), "the toolbar must still build:\n{snap}");
    assert!(ui.find("core:stuff").is_some(), "the material row is there:\n{snap}");
    assert!(
        ui.find("core:stuff.wood").is_some() && ui.find("core:stuff.stone").is_some(),
        "one button per material:\n{snap}"
    );
    assert!(snap.contains("wood ×12"), "stock is shown:\n{snap}");
    assert!(snap.contains("stone blocks · none"), "none is a fact, not a gap:\n{snap}");
    assert!(snap.contains("hp 140"), "the active material's stats are shown:\n{snap}");

    // Nothing selected: no row. The tree rebuilds on a clock, so time has
    // to move for the change to show.
    for t in &mut cv.tools {
        t.active = false;
    }
    cv.stuff.clear();
    for k in 1..=3 {
        frame(&mut ui, &sim, &cv, Input { time: k as f64, ..Default::default() });
    }
    assert!(ui.find("core:stuff").is_none(), "no row with nothing to choose for");
}
