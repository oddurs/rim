//! The interface: the HUD, the dock, the profiler, overlays and budgets.

use super::*;

/// 0171 the HUD is core's UI mod
pub(super) async fn the_hud_is_core_s_ui_mod(t: &mut T) {
    // A castaway on Auto gets a first-day hint over the map; a player would
    // read it and wave it away, and it would sit under clicks meant for the map.
    println!("\n# the first-day hint (5dc8f858)");
    t.check(t.app.ui.find("core:auto_hint").is_some(), "a castaway on Auto gets the first-day hint");
    t.click_ui("core:auto_hint.ok").await;
    t.check(t.app.ui.find("core:auto_hint").is_none(), "Got it waves it away");

    println!("\n# the HUD is a mod (0171)");
    for id in [
        "core:topbar",
        "core:clock",
        "core:status",
        "core:colonists",
        "core:toolbar",
        "core:messages",
        "core:inspector",
    ] {
        let found = t.app.ui.find(id).is_some();
        t.check(found, format!("core's UI draws '{id}'"));
    }
    t.check(t.app.ui.warnings().is_empty(), format!("core's UI loads cleanly ({:?})", t.app.ui.warnings()));
    let font = t.app.ui.info.font.clone();
    t.check(!font.is_empty(), format!("UI font: {font}"));
}

/// 0046 toolbar
pub(super) async fn toolbar(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before toolbar");
    println!("\n# toolbar (0046)");
    let keys: Vec<String> = t.app.tools.iter().map(|b| b.key.clone()).collect();
    let markable = (0..defs.designations.len()).filter(|&d| crate::markable(&defs, d as rim_sim::defs::DefId)).count();
    // Select, cancel, stockpile, clear zone and save as plan, besides one
    // per def and one per house plan.
    let n_expected = 5 + markable + defs.things.iter().filter(|d| d.build.is_some()).count() + defs.plans.len();
    t.check(keys.len() == n_expected, format!("one tool per markable designation and buildable def ({})", keys.len()));
    for k in &keys {
        t.click_tool(k).await;
        t.frame().await;
        let picked = t.app.tools.iter().find(|b| b.key == *k).is_some_and(|b| b.tool == t.app.tool);
        t.check(picked, format!("the dock picks '{k}'"));
    }
    // The dock is one row whatever is loaded, and every tray, group by
    // group, fits a 1280-point screen with room to spare.
    let dock = t.ui_rect("core:dock").map(|r| r[3]);
    t.check(dock.is_some_and(|h| h < 60.0), format!("the dock is one row ({dock:?})"));
    let mut widest = 0.0f32;
    let groups: Vec<String> = t.app.tools.iter().filter(|b| b.category == "build").map(|b| b.group.clone()).collect();
    for (key, category) in [(KeyCode::Q, "orders"), (KeyCode::B, "build"), (KeyCode::Z, "zones")] {
        t.clear_dock().await;
        t.key(key).await;
        t.frame().await;
        let group_list: Vec<String> = if category == "build" { groups.clone() } else { vec![String::new()] };
        for g in group_list {
            if !g.is_empty() {
                t.click_ui(&format!("core:dock.groups.{g}")).await;
                t.frame().await;
            }
            match t.ui_rect("core:dock.tray") {
                Some(r) => widest = widest.max(r[2]),
                None => t.check(false, format!("the {category} tray opens")),
            }
        }
    }
    t.clear_dock().await;
    t.click_tool("build:core:wall").await;
    t.check(widest > 0.0 && widest < 1280.0 - 64.0, format!("the widest tray fits at 1280 ({widest:.0} pt)"));
    t.key(KeyCode::Escape).await;
    t.frame().await;
    t.check(t.app.tool == Tool::Select, "Escape drops the tool");
    t.check(t.ui_rect("core:dock.tray").is_some(), "and brings its tray back");
    t.key(KeyCode::Escape).await;
    t.frame().await;
    t.check(t.ui_rect("core:dock.tray").is_none(), "a second Escape closes the tray");
    let selected = t.app.selected;
    let docked = (t.ui_rect("core:dock"), t.ui_rect("core:inspector"), t.ui_rect("core:colonists"));
    t.key(KeyCode::B).await;
    t.frame().await;
    t.check(t.ui_rect("core:dock.palette.build").is_some(), "B opens the Build palette");
    let now = (t.ui_rect("core:dock"), t.ui_rect("core:inspector"), t.ui_rect("core:colonists"));
    t.check(now == docked, format!("the tray floats: nothing docked moved ({docked:?} → {now:?})"));
    t.shot("build_tray").await;
    // Tab and Shift+Tab step through the open tray's groups (5689930d):
    // [ and ] are levels now.
    let shown = |t: &T| -> Vec<String> {
        t.app
            .tools
            .iter()
            .map(|b| b.key.clone())
            .filter(|k| t.ui_rect(&format!("core:toolbar.{k}")).is_some())
            .collect()
    };
    let first = shown(t);
    t.key(KeyCode::Tab).await;
    let second = shown(t);
    t.check(!second.is_empty() && second != first, "Tab shows the open tray's next group");
    t.check(t.app.selected == selected, "and leaves the selection alone");
    let raw =
        RawInput { keys: vec![KeyCode::Tab], pressed: vec!["shift+tab".into()], shift: true, ..Default::default() };
    t.input(RawInput { mouse: t.mouse, ..raw }).await;
    t.settle().await;
    t.check(shown(t) == first, "Shift+Tab goes back");
    t.key(KeyCode::Escape).await;
    t.frame().await;
    t.check(
        t.ui_rect("core:dock.palette.build").is_none() && t.app.selected == selected,
        "Escape closes an open palette before it touches the selection",
    );
}

/// 0048 HUD
pub(super) async fn hud(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before hud");
    let founder = carry.founder.expect("set before hud");
    let home = carry.home.expect("set before hud");
    println!("\n# messages, colonist bar, clock, speed (0048)");
    t.key(KeyCode::Space).await;
    t.check(t.app.paused, "space pauses");
    t.check(t.ui_text().contains("\"paused\""), "the clock says paused");
    // Paused, orders still land: planning while paused is how the game is
    // played. Nothing ticks here; the frames alone must apply them.
    let tick = t.w().tick;
    let (marked, planned) = (t.count::<(&Thing, &Designated)>(), t.count::<&Blueprint>());
    let oak = defs.thing_id("tree_oak").unwrap();
    let tree = t
        .w()
        .ecs
        .query::<(&Thing, Option<&Designated>)>()
        .iter()
        .filter(|(th, d)| th.def == oak && d.is_none())
        .map(|(th, _)| th.pos)
        .min_by_key(|p| (p.octile(home), p.x, p.y));
    t.click_tool("designate:core:chop").await;
    // Centred first, so the cell isn't under a panel whatever the map.
    if let Some(p) = tree {
        t.focus(p);
        t.drag(p, p).await;
    }
    let spot = open_square(t.w(), home.offset(10, 10), 2).unwrap_or(home.offset(10, 10));
    t.click_tool("build:core:wall").await;
    t.focus(spot);
    t.drag(spot, spot).await;
    t.frame().await;
    t.check(t.w().tick == tick, "paused: no time passed");
    t.check(tree.is_none() || t.count::<(&Thing, &Designated)>() > marked, "paused, a designation shows at once");
    t.check(t.count::<&Blueprint>() > planned, "paused, a plan shows at once");
    t.shot("paused_orders").await;
    // Taking a selected plan back while paused drops it from the selection
    // at once, not when time next runs.
    let plan = t
        .w()
        .ecs
        .query::<(Entity, &Thing)>()
        .with::<&Blueprint>()
        .iter()
        .find(|(_, th)| th.pos == spot)
        .map(|(e, _)| e);
    if let Some(plan) = plan {
        crate::select(&mut t.app, vec![plan]);
        t.app.sim.push(Command::Cancel { a: spot, b: spot });
        t.frame().await;
        t.check(t.app.selected.is_none(), "paused, a cancelled plan leaves the selection");
    } else {
        t.check(false, "paused, the wall plan is on its cell");
    }
    t.key(KeyCode::Escape).await;
    t.key(KeyCode::Space).await;
    t.check(!t.app.paused, "space resumes");
    for (k, want) in [(KeyCode::Key1, 1), (KeyCode::Key2, 3), (KeyCode::Key3, 6)] {
        t.key(k).await;
        t.check(t.app.speed == want, format!("speed key sets {want}x"));
    }
    t.frame().await;
    let snap = t.ui_text();
    t.check(snap.contains(&format!("\"Day {}\"", t.w().day() + 1)), "top bar shows the day");
    let hh = t.w().hour() as u32;
    t.check(snap.contains(&format!("\"{hh:02}:")), "top bar shows the hour");
    t.check(snap.contains("\"6×\""), "top bar shows the speed");

    let kinds = [MsgKind::Info, MsgKind::Good, MsgKind::Threat, MsgKind::Bad];
    for k in kinds {
        t.app.sim.world.message(format!("autotest {k:?} message"), k);
    }
    t.settle().await;
    t.check(t.ui_text().contains("autotest Threat message"), "messages show as toasts");
    let th = &t.app.ui.theme;
    let colours: Vec<[f32; 4]> = ["text", "good", "threat", "bad"].iter().map(|c| th.color[*c]).collect();
    let distinct = (0..4).all(|i| (0..4).all(|j| i == j || colours[i] != colours[j]));
    t.check(distinct, "each message kind has its own colour token");

    t.key(KeyCode::Escape).await;
    t.key(KeyCode::Escape).await;
    let name = t.pawn(founder).name.clone();
    t.click_ui(&format!("core:colonists.{name}")).await;
    t.check(t.app.selected == Some(founder), "clicking the colonist bar selects that colonist");
    let cols: Vec<Entity> = t.w().colonists().collect();
    t.key(KeyCode::Tab).await;
    t.check(t.app.selected.is_some_and(|s| cols.contains(&s)), "tab cycles colonists");
    t.shot("messages").await;
}

/// 0049 profiler
pub(super) async fn profiler(t: &mut T) {
    println!("\n# profiler (0049)");
    t.key(KeyCode::F3).await;
    t.check(t.app.show_profiler, "F3 opens the profiler");
    t.ticks(1200);
    for _ in 0..20 {
        t.frame().await;
    }
    let names: Vec<String> = t.app.sim.profile.entries.iter().map(|e| e.0.clone()).collect();
    for sys in ["tick", "pawns", "needs", "regions", "rooms", "fields", "wealth"] {
        t.check(names.iter().any(|n| n == sys), format!("profiler times system '{sys}'"));
    }
    t.check(names.iter().any(|n| n == "mod:core"), "profiler times core's scripts");
    t.check(t.app.ui.find("core:profiler.panel").is_some(), "the profiler panel is drawn");
    t.check(t.ui_text().contains("ui:core"), "and it times core's UI code too");
    t.shot("profiler").await;
    t.key(KeyCode::F3).await;
}

/// 0160 field overlay
pub(super) async fn field_overlay(t: &mut T, carry: &mut Carry) {
    let defs = carry.defs.clone().expect("set before field_overlay");
    let site = carry.site.expect("set before field_overlay");
    println!("\n# field overlay (0160)");
    let shown: Vec<usize> = (0..defs.fields.len()).filter(|&i| defs.fields[i].overlay).collect();
    let n = shown.len();
    t.check(n >= 2, format!("core defines field layers with overlays ({n})"));
    t.check(t.app.overlay.is_none(), "overlay starts off");
    let fire = defs.thing_id("campfire").unwrap();
    // Outdoors: inside an enclosed room the temperature is the room's own
    // value, not the outdoor air plus the fire (seed 37 put one in the hut).
    let spot = (0..40)
        .filter_map(|r| open_square(t.w(), site.offset(r, -r), 1))
        .find(|&p| !t.w().map.indoors(p))
        .expect("open ground outdoors for a fire");
    let _ = t.app.sim.world.spawn_fixture(fire, spot, false);
    t.ticks(1);
    for &i in &shown {
        t.key(KeyCode::O).await;
        t.check(t.app.overlay == Some(i), format!("O shows the '{}' overlay", defs.fields[i].label));
        t.focus(spot);
        t.shot(&format!("overlay_{}", defs.fields[i].id)).await;
    }
    t.key(KeyCode::O).await;
    t.check(t.app.overlay.is_none() && t.app.storage_overlay, "after the fields, O shows the storage overlay");
    t.shot("overlay_storage").await;
    t.key(KeyCode::O).await;
    t.check(t.app.overlay.is_none() && !t.app.storage_overlay, "O again turns the overlay off");
    let temp = defs.lookup("field", "temperature").unwrap() as usize;
    let near = t.w().fields.value(&defs, &t.w().map, temp, spot);
    let outside = t.w().fields.ambient(temp);
    t.check(near > outside, format!("the campfire warms its cell ({near:.1}° vs {outside:.1}° outside)"));
    t.check(t.ui_text().contains("°C outside"), "top bar shows the outdoor temperature");

    // Hover readout over the world.
    let sp = t.screen(spot);
    t.input(RawInput { mouse: sp, ..Default::default() }).await;
    t.frame().await;
    t.check(t.app.ui.find("core:hover").is_some(), "hovering the world shows the readout");
    t.check(t.ui_text().contains("campfire"), "and names what's there");
}

/// 0173 devtools
pub(super) async fn devtools(t: &mut T) {
    println!("\n# devtools (0173)");
    t.key(KeyCode::F12).await;
    t.frame().await;
    t.check(t.app.ui.find("core:devtools.panel").is_some(), "F12 opens devtools");
    if let Some(r) = t.ui_rect("core:dock.orders") {
        t.input(RawInput { mouse: (r[0] + r[2] / 2.0, r[1] + r[3] / 2.0), ..Default::default() }).await;
        t.frame().await;
    }
    let inspect = t.app.ui.info.inspect.clone();
    t.check(
        inspect.as_ref().is_some_and(|i| i.owner == "core" && i.path.starts_with("docked")),
        format!("pointing at the toolbar inspects it ({:?})", inspect.map(|i| (i.id, i.owner))),
    );
    t.shot("devtools").await;
    t.click_ui("core:devtools.outlines").await;
    t.check(t.app.ui.info.outlines, "the outlines toggle turns on layout outlines");
    t.shot("outlines").await;
    t.click_ui("core:devtools.outlines").await;
    t.click_ui("core:devtools.gallery").await;
    t.frame().await;
    t.check(t.app.ui.find("core:gallery.panel").is_some(), "the kit gallery opens");
    t.shot("gallery").await;
    t.click_ui("core:devtools.gallery").await;
    t.key(KeyCode::F12).await;
    t.frame().await;
    t.check(t.app.ui.find("core:devtools.panel").is_none(), "F12 closes devtools");
}

/// render cost
pub(super) async fn render_cost(t: &mut T) {
    println!("\n# render cost");
    let zoom0 = t.app.cam.zoom;
    t.act(Action::Zoom(0.01, 800.0, 480.0)); // all the way out: the most cells
    t.frame().await;
    // CPU cost of issuing the frame's drawing (not the GPU, not vsync).
    let t0 = std::time::Instant::now();
    render(&mut t.app);
    let frame_ms = t0.elapsed().as_secs_f64() * 1e3;
    t.frame().await;
    macroquad::telemetry::enable();
    macroquad::telemetry::capture_frame();
    t.frame().await;
    t.frame().await;
    let calls = macroquad::telemetry::drawcalls().len();
    macroquad::telemetry::disable();
    println!("zoomed out ({:.1} px/cell): {calls} draw calls, render {frame_ms:.2} ms (CPU)", t.app.cam.zoom);
    t.check(calls > 0, "the renderer's draw calls can be counted");
    t.act(Action::Zoom(zoom0 / t.app.cam.zoom, 800.0, 480.0));
    t.frame().await;
    macroquad::telemetry::enable();
    macroquad::telemetry::capture_frame();
    t.frame().await;
    t.frame().await;
    let calls = macroquad::telemetry::drawcalls().len();
    macroquad::telemetry::disable();
    println!("normal zoom, HUD open: {calls} draw calls");
    // Windows clip without ending the UI's draw call (3442707f): with as
    // many windows open as the core mod allows, each with its own clip and
    // scroll areas, the UI's only extra draw calls are for size (a mesh
    // holds 15k vertices; the kit gallery alone outgrows one), never for a
    // clip. Sheets are one at a time, so that is the last sheet, the
    // palette and the gallery; all six once sheets are apps (0e773a3e).
    let (alone, _) = crate::draw::ui_calls();
    let wins = ["core:stores", "core:work", "core:zones", "core:news", "core:palette", "core:gallery"];
    for w in wins {
        t.app.ui.open_window(w);
    }
    t.settle().await;
    let open = wins.iter().filter(|w| t.app.ui.is_open(w)).count();
    macroquad::telemetry::enable();
    macroquad::telemetry::capture_frame();
    t.frame().await;
    t.frame().await;
    let with = macroquad::telemetry::drawcalls().len();
    macroquad::telemetry::disable();
    let (ui_calls, full) = crate::draw::ui_calls();
    t.shot("three_windows").await;
    println!("normal zoom, HUD and {open} windows open: {with} draw calls; the UI {ui_calls} ({full} for size), {alone} with none open");
    t.check(
        open >= 3 && alone == 1 && ui_calls == full + 1,
        format!("{open} windows open split the UI's draw call only for size ({ui_calls} calls, {full} for size)"),
    );
    for w in wins {
        t.app.ui.close_window(w);
    }
    t.settle().await;
    t.check(wins.iter().all(|w| !t.app.ui.is_open(w)), "and they all close again");
}

/// UI budget, live
pub(super) async fn ui_budget_live(t: &mut T) {
    let (b, l, p) = (t.app.ui.info.build_us, t.app.ui.info.layout_us, t.app.ui.info.paint_us);
    println!(
        "\nui: build {b:.0} µs (when rebuilt) · layout {l:.0} µs · paint {p:.0} µs · {} nodes · {} rebuilds",
        t.app.ui.info.nodes, t.app.ui.builds
    );
}
