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
    let chop = ui.find("core:toolbar.designate:core:chop").unwrap();
    let actions = click(&mut ui, &sim, &mut cv, centre(chop));
    assert_eq!(actions, vec![UiAction::Tool("designate:core:chop".into())]);

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
fn a_mods_ui_sends_only_its_own_events() {
    let dir = scratch_mods(
        "send",
        &[(
            "sender",
            "",
            &[(
                "ui/send.luau",
                r#"
ui.define("sender:panel", function(view)
    return ui.row({ id = "sender:panel", bg = "surface", pad = 8,
        ui.row({ id = "sender:own", pad = 4, bg = "surface", on_click = function() act.send("sender:ping", { n = 1 }) end,
            ui.text({ "own" }) }),
        ui.row({ id = "sender:other", pad = 4, bg = "surface", on_click = function() act.send("weather:force", { id = "storm" }) end,
            ui.text({ "other" }) }),
    })
end)
ui.mount("windows", "sender:panel")
"#,
            )],
        )],
    );
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let at = centre(ui.find("sender:own").unwrap());
    let own = click(&mut ui, &sim, &mut cv, at);
    assert!(own.iter().any(|a| matches!(a, rim_ui::view::UiAction::Send(n, _) if n == "sender:ping")), "{own:?}");
    let at = centre(ui.find("sender:other").unwrap());
    let other = click(&mut ui, &sim, &mut cv, at);
    assert!(!other.iter().any(|a| matches!(a, rim_ui::view::UiAction::Send(..))), "can't speak for another mod");
    assert!(ui.warnings().iter().any(|w| w.contains("can only send its own events")), "{:?}", ui.warnings());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Two windows a probe mod declares open, each with a line of text inside.
const TWO_WINDOWS: &str = r#"
ui.window("probe:a", { title = "Alpha", w = 300, h = 200, resizable = true, open = true }, function(view)
    return ui.text({ "alpha body", id = "probe:a.body" })
end)
ui.window("probe:b", { title = "Beta", w = 300, h = 200, open = true }, function(view)
    return ui.text({ "beta body", id = "probe:b.body" })
end)
"#;

fn drag(ui: &mut rim_ui::Ui, sim: &rim_sim::Sim, cv: &rim_ui::view::ClientView, from: (f32, f32), by: (f32, f32)) {
    let to = (from.0 + by.0, from.1 + by.1);
    frame(ui, sim, cv, Input { mouse: from, time: 1.0, ..Default::default() });
    frame(ui, sim, cv, Input { mouse: from, left_pressed: true, time: 2.0, ..Default::default() });
    frame(ui, sim, cv, Input { mouse: to, time: 3.0, ..Default::default() });
    frame(ui, sim, cv, Input { mouse: to, left_released: true, time: 4.0, ..Default::default() });
    frame(ui, sim, cv, Input { mouse: to, time: 5.0, ..Default::default() });
}

#[test]
fn windows_drag_resize_close_and_stack() {
    let dir = scratch_mods("windows", &[("probe", "", &[("ui/win.luau", TWO_WINDOWS)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    assert_eq!(ui.window_order(), ["probe:a", "probe:b"], "declared open, in order");
    let a = ui.window_rect("probe:a").expect("alpha is placed");
    let b = ui.window_rect("probe:b").expect("beta is placed");
    assert_eq!((a[2], a[3]), (300.0, 200.0), "sized as declared");
    assert_eq!((b[0] - a[0], b[1] - a[1]), (24.0, 24.0), "the second cascades from the first");
    assert!(ui.find("probe:a.body").is_some() && ui.find("probe:b.body").is_some(), "both bodies show");
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());

    // Stacking: a press on alpha's body (under beta's title bar? no: alpha
    // is to the upper left, so press its own corner) raises it above beta.
    let title = ui.find("probe:a.title").unwrap();
    click(&mut ui, &sim, &mut cv, (title[0] + 10.0, title[1] + 10.0));
    assert_eq!(ui.window_order(), ["probe:b", "probe:a"], "the pressed window comes to the front");
    assert!(ui.take_layout_dirty(), "stacking is part of the layout");

    // Drag by the title bar.
    let title = ui.find("probe:a.title").unwrap();
    drag(&mut ui, &sim, &cv, (title[0] + 10.0, title[1] + 10.0), (50.0, 30.0));
    let moved = ui.window_rect("probe:a").unwrap();
    assert_eq!((moved[0] - a[0], moved[1] - a[1]), (50.0, 30.0), "moved by the drag");
    assert!(ui.take_layout_dirty());
    // The body moved with it and stays clickable.
    let body = ui.find("probe:a.body").unwrap();
    assert!(body[0] > moved[0] && body[1] > moved[1] + 20.0, "body inside the window, below the title");

    // Resize by the grip.
    let grip = ui.find("probe:a.resize").expect("alpha is resizable");
    drag(&mut ui, &sim, &cv, centre(grip), (40.0, 20.0));
    let grown = ui.window_rect("probe:a").unwrap();
    assert_eq!((grown[2], grown[3]), (340.0, 220.0), "grew by the drag");
    assert!(ui.find("probe:b.resize").is_none(), "beta declared no resize");

    // Never off screen: a drag far past the edge keeps the title bar reachable.
    let title = ui.find("probe:a.title").unwrap();
    drag(&mut ui, &sim, &cv, (title[0] + 10.0, title[1] + 10.0), (5000.0, 5000.0));
    let r = ui.window_rect("probe:a").unwrap();
    assert!(r[0] <= 1600.0 - 80.0 && r[1] <= 960.0 - 40.0, "{r:?} stays reachable");

    // Close by the button.
    let close = ui.find("probe:a.close").unwrap();
    click(&mut ui, &sim, &mut cv, centre(close));
    frame(&mut ui, &sim, &cv, Input { time: 9.0, ..Default::default() });
    assert!(!ui.is_open("probe:a"));
    assert!(ui.window_rect("probe:a").is_none() && ui.find("probe:a.body").is_none(), "closed windows are not built");
    assert_eq!(ui.window_order(), ["probe:b"]);
    // Reopened from the engine (a handler would use ui.open), it is back on top.
    ui.open_window("probe:a");
    frame(&mut ui, &sim, &cv, Input { time: 10.0, ..Default::default() });
    assert_eq!(ui.window_order(), ["probe:b", "probe:a"]);
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_window_layout_survives_a_restart_and_a_mod_update() {
    let dir = scratch_mods("winlayout", &[("probe", "", &[("ui/win.luau", TWO_WINDOWS)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let title = ui.find("probe:a.title").unwrap();
    drag(&mut ui, &sim, &cv, (title[0] + 10.0, title[1] + 10.0), (-100.0, -60.0));
    let grip = ui.find("probe:a.resize").unwrap();
    drag(&mut ui, &sim, &cv, centre(grip), (60.0, 40.0));
    let close = ui.find("probe:b.close").unwrap();
    click(&mut ui, &sim, &mut cv, centre(close));
    frame(&mut ui, &sim, &cv, Input { time: 20.0, ..Default::default() });
    let a = ui.window_rect("probe:a").unwrap();
    let saved = ui.layout_toml();
    assert!(saved.contains("[window.\"probe:a\"]"), "{saved}");
    let _ = std::fs::remove_dir_all(&dir);

    // A "mod update": alpha's default size changed and beta now opens by
    // default; the saved layout still wins on both.
    let updated = TWO_WINDOWS.replace("w = 300, h = 200, resizable = true", "w = 500, h = 400, resizable = true");
    let dir = scratch_mods("winlayout2", &[("probe", "", &[("ui/win.luau", &updated)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    ui.restore_layout(&saved).expect("the layout parses");
    frame(&mut ui, &sim, &cv, Default::default());
    assert_eq!(ui.window_rect("probe:a"), Some(a), "alpha is where it was, at the size it was");
    assert!(!ui.is_open("probe:b"), "beta stays closed");
    assert!(!ui.take_layout_dirty(), "restoring is not a change");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn two_mods_declaring_one_window_is_reported() {
    let dir = scratch_mods(
        "winconflict",
        &[
            (
                "one",
                "",
                &[(
                    "ui/win.luau",
                    "ui.window(\"shared:win\", { title = \"One\" }, function(view) return ui.text({ \"one\" }) end)",
                )],
            ),
            (
                "two",
                "",
                &[(
                    "ui/win.luau",
                    "ui.window(\"shared:win\", { title = \"Two\" }, function(view) return ui.text({ \"two\" }) end)",
                )],
            ),
        ],
    );
    let sim = sim_at(&dir);
    let ui = ui_for(&sim);
    let w = ui.warnings();
    assert!(
        w.iter().any(|w| w.starts_with("UI conflict: window 'shared:win' declared by")),
        "a shared window id is reported: {w:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_devtools_toggle_opens_the_gallery_window() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.show_devtools = true;
    frame(&mut ui, &sim, &cv, Default::default());
    assert!(!ui.is_open("core:gallery"));
    let toggle = ui.find("core:devtools.gallery").expect("the gallery toggle");
    click(&mut ui, &sim, &mut cv, centre(toggle));
    frame(&mut ui, &sim, &cv, Input { time: 1.0, ..Default::default() });
    assert!(ui.is_open("core:gallery"));
    assert!(ui.find("core:gallery.panel").is_some(), "the gallery shows in its window");
    assert!(ui.find("core:gallery.resize").is_some());
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    let close = ui.find("core:gallery.close").unwrap();
    click(&mut ui, &sim, &mut cv, centre(close));
    frame(&mut ui, &sim, &cv, Input { time: 2.0, ..Default::default() });
    assert!(!ui.is_open("core:gallery"));
}

/// A solid PNG of one colour, `size` px square, as bytes.
fn solid_png(size: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, size, size);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().unwrap();
        let px: Vec<u8> = (0..size * size).flat_map(|_| rgba).collect();
        w.write_image_data(&px).unwrap();
    }
    out
}

/// A mods dir with a probe mod shipping `probe:dot` at 1x (4 px) and 2x
/// (8 px), and a component showing it plain, tinted, and one that is missing.
fn image_mods(name: &str) -> std::path::PathBuf {
    let dir = scratch_mods(
        name,
        &[(
            "probe",
            "",
            &[(
                "ui/img.luau",
                r#"
ui.define("probe:pics", function(view)
    return ui.row({ id = "probe:pics", gap = 0, pad = 0,
        ui.image({ id = "probe:plain", src = "probe:dot" }),
        ui.image({ id = "probe:tinted", src = "probe:dot", tint = true }),
        ui.image({ id = "probe:accent", src = "probe:dot", tint = true, color = "accent", w = 10, h = 10 }),
        ui.image({ id = "probe:missing", src = "probe:nothing" }),
    })
end)
ui.mount("top", "probe:pics", { order = 90 })
"#,
            )],
        )],
    );
    let img = dir.join("probe").join("ui").join("img");
    std::fs::create_dir_all(&img).unwrap();
    std::fs::write(img.join("dot.png"), solid_png(4, [255, 0, 0, 255])).unwrap();
    std::fs::write(img.join("dot@2x.png"), solid_png(8, [0, 255, 0, 255])).unwrap();
    dir
}

/// The image quads drawn last frame, by the rect they cover.
fn image_quads(out: &rim_ui::Output) -> Vec<(rim_ui::text::GlyphQuad, [f32; 4])> {
    out.draw
        .iter()
        .filter_map(|d| match d {
            Draw::Glyphs { quads, color } if quads.len() == 1 && quads[0].uv[2] >= 4.0 && quads[0].uv[2] <= 8.0 => {
                Some((quads[0], *color))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_mods_png_draws_at_1x_and_2x_with_no_client_change() {
    let dir = image_mods("img1x");
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    let out = frame(&mut ui, &sim, &cv, Default::default());
    let r = ui.find("probe:plain").expect("the image is laid out");
    assert_eq!((r[2], r[3]), (4.0, 4.0), "its own size at 1x");
    let quads = image_quads(&out);
    let plain = quads.iter().find(|(q, _)| q.dst == r).expect("a quad over the image");
    assert_eq!((plain.0.uv[2], plain.0.uv[3]), (4.0, 4.0), "the 1x pixels");
    assert!(plain.0.color, "a plain image keeps its own colours");
    // The atlas holds the red pixels where the quad points.
    let a = &ui.text.atlas;
    let i = ((plain.0.uv[1] as u32) * a.size + plain.0.uv[0] as u32) as usize * 4;
    assert_eq!(&a.pixels[i..i + 4], &[255, 0, 0, 255]);
    assert!(ui.warnings().iter().all(|w| !w.contains("'probe:dot'")), "{:?}", ui.warnings());

    // At 2x the @2x file is used: 8 atlas pixels over 8 screen pixels.
    let mut ui2 = rim_ui::Ui::new(rim_ui::mods_of(&sim), 2.0, 1.0).unwrap();
    let cv2 = rim_ui::view::ClientView { scale: 2.0, ..client(&sim) };
    let out2 = frame(&mut ui2, &sim, &cv2, Default::default());
    let r2 = ui2.find("probe:plain").unwrap();
    assert_eq!((r2[2], r2[3]), (8.0, 8.0), "4 logical px is 8 physical");
    let q2 = image_quads(&out2).into_iter().find(|(q, _)| q.dst == r2).expect("a quad at 2x");
    assert_eq!((q2.0.uv[2], q2.0.uv[3]), (8.0, 8.0), "the 2x pixels");
    let a = &ui2.text.atlas;
    let i = ((q2.0.uv[1] as u32) * a.size + q2.0.uv[0] as u32) as usize * 4;
    assert_eq!(&a.pixels[i..i + 4], &[0, 255, 0, 255], "the @2x file's pixels");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_image_is_a_named_warning_and_a_placeholder() {
    let dir = image_mods("imgmissing");
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let w = ui.warnings();
    assert!(
        w.iter().any(|w| w.contains("image 'probe:nothing' not found") && w.contains("ui/img/nothing.png")),
        "named: {w:?}"
    );
    assert!(ui.find("probe:plain").is_some(), "the rest of the row still shows");
    let snap = ui.snapshot();
    assert!(snap.contains("probe:nothing"), "a placeholder stands where the image would be:\n{snap}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tinted_icons_follow_the_text_colour() {
    let dir = image_mods("imgtint");
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    let out = frame(&mut ui, &sim, &cv, Default::default());
    let quads = image_quads(&out);
    let text = ui.theme.color("text").unwrap();
    let accent = ui.theme.color("accent").unwrap();
    let tinted = ui.find("probe:tinted").unwrap();
    let (q, c) = quads.iter().find(|(q, _)| q.dst == tinted).expect("the tinted quad");
    assert!(!q.color, "a tinted image is a mask, like a glyph");
    assert_eq!(*c, text, "in the theme's text colour");
    let acc = ui.find("probe:accent").unwrap();
    assert_eq!((acc[2], acc[3]), (10.0, 10.0), "w/h override the image's size");
    let (_, c) = quads.iter().find(|(q, _)| q.dst == acc).expect("the accent quad");
    assert_eq!(*c, accent, "color picks the tint");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A probe mod with a text input and a slider, each reporting into state.
const INPUT_MOD: &str = r#"
local kit = require("@core/ui/kit")
ui.define("probe:form", function(view)
    return ui.row({ id = "probe:form", gap = "s", align = "center",
        kit.input({ id = "probe:name", value = "ab", on_change = function(text)
            ui.set_state("probe:changed", text)
        end, on_submit = function(text)
            ui.set_state("probe:submitted", text)
        end }),
        ui.text({ "changed=" .. ui.state("probe:changed", "-"), id = "probe:changed" }),
        ui.text({ "submitted=" .. ui.state("probe:submitted", "-"), id = "probe:submitted" }),
        kit.slider({ id = "probe:slider", value = ui.state("probe:v", 0), on_change = function(v)
            ui.set_state("probe:v", v)
        end }),
        ui.text({ string.format("v=%.2f", ui.state("probe:v", 0)), id = "probe:v" }),
    })
end)
ui.mount("top", "probe:form", { order = 90 })
"#;

fn type_keys(
    ui: &mut rim_ui::Ui,
    sim: &rim_sim::Sim,
    cv: &rim_ui::view::ClientView,
    t: &mut f64,
    keys: &[rim_ui::Key],
    shift: bool,
) -> rim_ui::Output {
    *t += 0.1;
    frame(ui, sim, cv, Input { keys: keys.to_vec(), shift, time: *t, ..Default::default() })
}

#[test]
fn typing_survives_twenty_rebuilds_without_losing_the_caret() {
    use rim_ui::Key;
    let dir = scratch_mods("input20", &[("probe", "", &[("ui/form.luau", INPUT_MOD)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let name = ui.find("probe:name").expect("the input is laid out");
    click(&mut ui, &sim, &mut cv, centre(name));
    assert_eq!(ui.focused_id().as_deref(), Some("probe:name"));
    let mut t = 1.0;
    let builds = ui.builds;
    for c in "cdef".chars() {
        type_keys(&mut ui, &sim, &cv, &mut t, &[Key::Char(c)], false);
    }
    for _ in 0..20 {
        type_keys(&mut ui, &sim, &cv, &mut t, &[], false);
    }
    assert!(ui.builds - builds >= 20, "the tree was rebuilt {} times", ui.builds - builds);
    let e = ui.edit_state("probe:name").unwrap();
    assert_eq!((e.text.as_str(), e.caret), ("abcdef", 6), "the buffer and caret are the engine's");
    assert!(ui.snapshot().contains("input #probe:name \"abcdef\""), "{}", ui.snapshot());
    assert!(ui.snapshot().contains("changed=abcdef"), "on_change ran: {}", ui.snapshot());
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn focus_edit_submit_and_escape() {
    use rim_ui::Key;
    let dir = scratch_mods("inputkeys", &[("probe", "", &[("ui/form.luau", INPUT_MOD)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let mut t = 1.0;
    // Unfocused: keys are the game's.
    let out = type_keys(&mut ui, &sim, &cv, &mut t, &[Key::Char('x')], false);
    assert!(!out.captured_keys && ui.edit_state("probe:name").is_none());
    // Focus by click: the caret sits at the end of the value.
    let name = ui.find("probe:name").unwrap();
    click(&mut ui, &sim, &mut cv, centre(name));
    assert_eq!(ui.edit_state("probe:name").unwrap().caret, 2);
    // Edit: home, delete, type in the middle, select with shift, replace.
    let out = type_keys(&mut ui, &sim, &cv, &mut t, &[Key::Home, Key::Delete, Key::Char('Z')], false);
    assert!(out.captured_keys, "a focused input takes the keys");
    assert_eq!(ui.edit_state("probe:name").unwrap().text, "Zb");
    type_keys(&mut ui, &sim, &cv, &mut t, &[Key::End], false);
    type_keys(&mut ui, &sim, &cv, &mut t, &[Key::Left], true);
    assert_eq!(ui.edit_state("probe:name").unwrap().selection(), (1, 2));
    type_keys(&mut ui, &sim, &cv, &mut t, &[Key::Char('q'), Key::Char('r')], false);
    assert_eq!(ui.edit_state("probe:name").unwrap().text, "Zqr");
    type_keys(&mut ui, &sim, &cv, &mut t, &[Key::Backspace], false);
    assert_eq!(ui.edit_state("probe:name").unwrap().text, "Zq");
    // A caret and the text are painted inside the box.
    let out = type_keys(&mut ui, &sim, &cv, &mut t, &[], false);
    let name = ui.find("probe:name").unwrap();
    let caret = out.draw.iter().any(|d| match d {
        Draw::Rect { rect, .. } => {
            rect[2] <= 2.0 && rect[0] > name[0] && rect[0] < name[0] + name[2] && rect[1] >= name[1]
        }
        _ => false,
    });
    assert!(caret, "a thin caret rect inside the input");
    // Submit.
    t += 0.1;
    let out = frame(&mut ui, &sim, &cv, Input { enter: true, time: t, ..Default::default() });
    assert!(out.captured_keys);
    type_keys(&mut ui, &sim, &cv, &mut t, &[], false);
    assert!(ui.snapshot().contains("submitted=Zq"), "{}", ui.snapshot());
    assert_eq!(ui.focused_id().as_deref(), Some("probe:name"), "submit keeps focus");
    // Escape gives the keyboard back; the buffer stays.
    type_keys(&mut ui, &sim, &cv, &mut t, &[Key::Escape], false);
    assert_eq!(ui.focused_id(), None);
    let out = type_keys(&mut ui, &sim, &cv, &mut t, &[Key::Char('!')], false);
    assert!(!out.captured_keys);
    assert_eq!(ui.edit_state("probe:name").unwrap().text, "Zq");
    // Tab away from a focused input moves focus like any control.
    click(&mut ui, &sim, &mut cv, centre(name));
    t += 0.1;
    let out = frame(&mut ui, &sim, &cv, Input { tab: true, time: t, ..Default::default() });
    assert!(out.captured_keys && ui.focused_id().as_deref() != Some("probe:name"));
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hot_reload_keeps_the_buffer_of_a_focused_input() {
    use rim_ui::Key;
    let dir = scratch_mods("inputreload", &[("probe", "", &[("ui/form.luau", INPUT_MOD)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let name = ui.find("probe:name").unwrap();
    click(&mut ui, &sim, &mut cv, centre(name));
    let mut t = 1.0;
    type_keys(&mut ui, &sim, &cv, &mut t, &[Key::Char('c')], false);
    assert!(ui.reload_now(), "the UI reloads");
    type_keys(&mut ui, &sim, &cv, &mut t, &[], false);
    assert_eq!(ui.edit_state("probe:name").unwrap().text, "abc");
    assert!(ui.snapshot().contains("input #probe:name \"abc\""), "{}", ui.snapshot());
    assert_eq!(ui.focused_id().as_deref(), Some("probe:name"), "still focused after the reload");
    type_keys(&mut ui, &sim, &cv, &mut t, &[Key::Char('d')], false);
    assert_eq!(ui.edit_state("probe:name").unwrap().text, "abcd");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_slider_reports_the_drag() {
    let dir = scratch_mods("slider", &[("probe", "", &[("ui/form.luau", INPUT_MOD)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let s = ui.find("probe:slider").unwrap();
    let at = |f: f32| (s[0] + s[2] * f, s[1] + s[3] / 2.0);
    frame(&mut ui, &sim, &cv, Input { mouse: at(0.25), left_pressed: true, time: 1.0, ..Default::default() });
    frame(&mut ui, &sim, &cv, Input { mouse: at(0.25), time: 1.1, ..Default::default() });
    assert!(ui.snapshot().contains("v=0.25"), "{}", ui.snapshot());
    frame(&mut ui, &sim, &cv, Input { mouse: at(0.75), time: 1.2, ..Default::default() });
    frame(&mut ui, &sim, &cv, Input { mouse: at(0.75), left_released: true, time: 1.3, ..Default::default() });
    frame(&mut ui, &sim, &cv, Input { mouse: at(0.75), time: 1.4, ..Default::default() });
    assert!(ui.snapshot().contains("v=0.75"), "{}", ui.snapshot());
    // Released: moving on changes nothing.
    frame(&mut ui, &sim, &cv, Input { mouse: at(0.1), time: 1.5, ..Default::default() });
    frame(&mut ui, &sim, &cv, Input { mouse: at(0.1), time: 1.6, ..Default::default() });
    assert!(ui.snapshot().contains("v=0.75"), "{}", ui.snapshot());
    let _ = std::fs::remove_dir_all(&dir);
}

/// A probe mod binding an action that counts its own runs.
const BIND_MOD: &str = r#"
ui.bind("probe:hello", { key = "h", label = "Say hello" }, function()
    ui.set_state("probe:fired", ui.state("probe:fired", 0) + 1)
end)
ui.define("probe:count", function(view)
    return ui.text({ "fired=" .. ui.state("probe:fired", 0), id = "probe:count" })
end)
ui.mount("top", "probe:count", { order = 90 })
"#;

fn press(
    ui: &mut rim_ui::Ui,
    sim: &rim_sim::Sim,
    cv: &rim_ui::view::ClientView,
    t: &mut f64,
    key: &str,
) -> rim_ui::Output {
    *t += 0.1;
    let out = frame(ui, sim, cv, Input { pressed: vec![key.to_string()], time: *t, ..Default::default() });
    *t += 0.1;
    frame(ui, sim, cv, Input { time: *t, ..Default::default() });
    out
}

#[test]
fn a_mods_bound_action_fires_from_its_key_and_from_the_palette() {
    let dir = scratch_mods("bindkey", &[("probe", "", &[("ui/bind.luau", BIND_MOD)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let mut t = 1.0;
    let out = press(&mut ui, &sim, &cv, &mut t, "h");
    assert!(out.captured_keys, "a bound key is the UI's");
    assert!(ui.snapshot().contains("fired=1"), "{}", ui.snapshot());
    let out = press(&mut ui, &sim, &cv, &mut t, "j");
    assert!(!out.captured_keys, "an unbound key is the game's");
    assert!(ui.snapshot().contains("fired=1"));

    // The palette: Ctrl+K opens it with the query focused; typing filters;
    // Enter runs the first match and closes it.
    press(&mut ui, &sim, &cv, &mut t, "ctrl+k");
    assert!(ui.is_open("core:palette"), "the palette opened");
    assert_eq!(ui.focused_id().as_deref(), Some("core:palette.query"), "the query has the keyboard");
    assert!(ui.find("core:palette.probe:hello").is_some(), "the action is listed");
    assert!(ui.find("core:palette.core:pause").is_some(), "core's keys are listed too");
    let out = press(&mut ui, &sim, &cv, &mut t, "h");
    assert!(out.captured_keys && ui.snapshot().contains("fired=1"), "keys type into the query, not into bindings");
    for c in "ello".chars() {
        t += 0.1;
        frame(&mut ui, &sim, &cv, Input { keys: vec![rim_ui::Key::Char(c)], time: t, ..Default::default() });
    }
    t += 0.1;
    frame(&mut ui, &sim, &cv, Input { time: t, ..Default::default() });
    assert!(ui.find("core:palette.probe:hello").is_some() && ui.find("core:palette.core:pause").is_none(), "filtered");
    t += 0.1;
    frame(&mut ui, &sim, &cv, Input { enter: true, time: t, ..Default::default() });
    t += 0.1;
    frame(&mut ui, &sim, &cv, Input { time: t, ..Default::default() });
    assert!(ui.snapshot().contains("fired=2"), "Enter ran the first match: {}", ui.snapshot());
    assert!(!ui.is_open("core:palette"), "and closed the palette");
    // Clicking a row runs it too, once filtered into view: the full list
    // is longer than the palette.
    press(&mut ui, &sim, &cv, &mut t, "ctrl+k");
    for c in "hello".chars() {
        t += 0.1;
        frame(&mut ui, &sim, &cv, Input { keys: vec![rim_ui::Key::Char(c)], time: t, ..Default::default() });
    }
    t += 0.1;
    frame(&mut ui, &sim, &cv, Input { time: t, ..Default::default() });
    let row = ui.find("core:palette.probe:hello").unwrap();
    click(&mut ui, &sim, &mut cv, centre(row));
    t += 0.1;
    frame(&mut ui, &sim, &cv, Input { time: t, ..Default::default() });
    assert!(ui.snapshot().contains("fired=3"), "{}", ui.snapshot());
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_palette_moves_its_selection_with_the_arrows_and_starts_clean() {
    use rim_ui::Key;
    let dir = scratch_mods("palettesel", &[("probe", "", &[("ui/bind.luau", BIND_MOD)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let mut t = 1.0;
    press(&mut ui, &sim, &cv, &mut t, "ctrl+k");
    // The second row is core:speed1; Down once selects it, Enter runs it.
    t += 0.1;
    let out = frame(&mut ui, &sim, &cv, Input { keys: vec![Key::Down], time: t, ..Default::default() });
    assert!(out.captured_keys);
    t += 0.1;
    frame(&mut ui, &sim, &cv, Input { time: t, ..Default::default() });
    assert_eq!(ui.edit_state("core:palette.query").map(|e| e.text.as_str()), Some(""), "the arrow typed nothing");
    t += 0.1;
    let out = frame(&mut ui, &sim, &cv, Input { enter: true, time: t, ..Default::default() });
    t += 0.1;
    frame(&mut ui, &sim, &cv, Input { time: t, ..Default::default() });
    assert!(out.actions.iter().any(|a| matches!(a, UiAction::Speed(1))), "ran the selected row: {:?}", out.actions);
    assert!(!ui.is_open("core:palette"));
    // Reopened, the query is empty and the selection at the top, whatever was typed before.
    press(&mut ui, &sim, &cv, &mut t, "ctrl+k");
    for c in "pau".chars() {
        t += 0.1;
        frame(&mut ui, &sim, &cv, Input { keys: vec![Key::Char(c)], time: t, ..Default::default() });
    }
    assert_eq!(ui.edit_state("core:palette.query").map(|e| e.text.as_str()), Some("pau"));
    ui.close_window("core:palette");
    t += 0.1;
    frame(&mut ui, &sim, &cv, Input { time: t, ..Default::default() });
    assert!(!ui.is_open("core:palette"));
    press(&mut ui, &sim, &cv, &mut t, "ctrl+k");
    assert!(ui.is_open("core:palette"));
    assert_eq!(ui.edit_state("core:palette.query").map(|e| e.text.as_str()), Some(""), "starts clean");
    assert!(ui.find("core:palette.core:speed1").is_some(), "unfiltered again");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_text_leaf_measures_and_draws_inside_its_padding() {
    let dir = scratch_mods(
        "textpad",
        &[(
            "probe",
            "",
            &[(
                "ui/pad.luau",
                r#"
ui.define("probe:pad", function(view)
    return ui.row({ gap = 0, pad = 0, align = "start",
        ui.text({ "plain", id = "probe:plain" }),
        ui.text({ "padded", id = "probe:padded", pad = 6, bg = "surface" }),
        ui.input({ id = "probe:in", value = "typed", pad = 6, padx = 10 }),
    })
end)
ui.mount("windows", "probe:pad")
"#,
            )],
        )],
    );
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    let out = frame(&mut ui, &sim, &cv, Default::default());
    let plain = ui.find("probe:plain").unwrap();
    let padded = ui.find("probe:padded").unwrap();
    let input = ui.find("probe:in").unwrap();
    assert_eq!(padded[3], plain[3] + 12.0, "padding is part of the box's height");
    assert_eq!(input[3], plain[3] + 12.0);
    assert!(input[2] > plain[2] + 20.0, "and its width");
    // The glyphs of the padded text start inside the padding.
    let first_glyph_x = out
        .draw
        .iter()
        .filter_map(|d| match d {
            Draw::Glyphs { quads, .. } => quads.first().map(|q| q.dst),
            _ => None,
        })
        .filter(|q| {
            q[1] >= padded[1] && q[1] < padded[1] + padded[3] && q[0] >= padded[0] && q[0] < padded[0] + padded[2]
        })
        .map(|q| q[0])
        .fold(f32::MAX, f32::min);
    assert!(first_glyph_x >= padded[0] + 6.0, "text starts after the left padding: {first_glyph_x} vs {}", padded[0]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn two_mods_binding_one_key_is_reported() {
    let dir = scratch_mods(
        "bindconflict",
        &[
            ("one", "", &[("ui/k.luau", "ui.bind(\"one:a\", { key = \"Shift+X\" }, function() end)")]),
            ("two", "", &[("ui/k.luau", "ui.bind(\"two:b\", { key = \"shift + x\" }, function() end)\nui.bind(\"one:a\", { key = \"y\" }, function() end)")]),
        ],
    );
    let sim = sim_at(&dir);
    let ui = ui_for(&sim);
    let w = ui.warnings();
    assert!(w.iter().any(|w| w.starts_with("UI conflict: key 'shift+x' bound by")), "one key, two actions: {w:?}");
    assert!(
        w.iter().any(|w| w.starts_with("UI conflict: action 'one:a' bound by 'one' and 'two'")),
        "one id, two mods: {w:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_keybinds_file_overrides_a_default_and_survives_a_restart() {
    let dir = scratch_mods("bindfile", &[("probe", "", &[("ui/bind.luau", BIND_MOD)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    ui.restore_keybinds("[keys]\n\"probe:hello\" = \"J\"\n").unwrap();
    frame(&mut ui, &sim, &cv, Default::default());
    let mut t = 1.0;
    press(&mut ui, &sim, &cv, &mut t, "h");
    assert!(ui.snapshot().contains("fired=0"), "the default key no longer fires");
    press(&mut ui, &sim, &cv, &mut t, "j");
    assert!(ui.snapshot().contains("fired=1"), "the player's key does");
    // Rebinding from the engine, and back to the default, round-trips.
    ui.rebind("probe:hello", Some("ctrl+H"));
    assert!(ui.take_keys_dirty());
    let saved = ui.keybinds_toml();
    assert!(saved.contains("\"probe:hello\" = \"ctrl+h\""), "{saved}");
    ui.rebind("probe:hello", Some("h"));
    assert!(!ui.keybinds_toml().contains("probe:hello"), "the default is not an override");
    // A restart: a fresh engine restores the file and the key holds.
    let mut ui2 = ui_for(&sim);
    ui2.restore_keybinds(&saved).unwrap();
    frame(&mut ui2, &sim, &cv, Default::default());
    press(&mut ui2, &sim, &cv, &mut t, "h");
    assert!(ui2.snapshot().contains("fired=0"));
    press(&mut ui2, &sim, &cv, &mut t, "ctrl+h");
    assert!(ui2.snapshot().contains("fired=1"), "{}", ui2.snapshot());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_table_sorts_by_a_clicked_column_and_keeps_its_rows_virtual() {
    let dir = scratch_mods(
        "table",
        &[(
            "probe",
            "",
            &[(
                "ui/table.luau",
                r#"
local kit = require("@core/ui/kit")
local rows = {}
for i = 1, 300 do
    rows[i] = { name = "pawn" .. string.format("%03d", i), age = (i * 37) % 90 }
end
ui.define("probe:table", function(view)
    return kit.table({ id = "probe:t", h = 100, row_h = 20, rows = rows, columns = {
        { label = "Name", w = 120, key = "name" },
        { label = "Age", w = 60, key = "age" },
        { label = "Plain", w = 60, value = function(rec) return rec.age * 2 end, sort = false },
    } })
end)
ui.mount("windows", "probe:table")
"#,
            )],
        )],
    );
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    frame(&mut ui, &sim, &cv, Input { time: 1.0, ..Default::default() });
    let first_rows = |ui: &rim_ui::Ui| -> Vec<String> {
        let snap = ui.snapshot();
        let sec = snap.split("== probe:table").nth(1).unwrap_or("");
        sec.lines().filter(|l| l.contains("\"pawn")).take(3).map(|l| l.trim().to_string()).collect()
    };
    let built = ui.snapshot().matches("\"pawn").count();
    assert!(built < 40, "{built} of 300 rows built for a 100 px window");
    assert!(first_rows(&ui)[0].contains("pawn001"), "unsorted: declaration order {:?}", first_rows(&ui));
    // Click the Age header: ascending; again: descending.
    let age = ui.find("probe:t.head.2").expect("the header is laid out");
    click(&mut ui, &sim, &mut cv, centre(age));
    frame(&mut ui, &sim, &cv, Input { time: 2.0, ..Default::default() });
    let ages = |ui: &rim_ui::Ui| -> Vec<i32> {
        let snap = ui.snapshot();
        let sec = snap.split("== probe:table").nth(1).unwrap_or("");
        let mut v = Vec::new();
        let lines: Vec<&str> = sec.lines().collect();
        for (i, l) in lines.iter().enumerate() {
            if l.contains("\"pawn") {
                if let Some(n) = lines.get(i + 2).and_then(|n| n.split('"').nth(1)).and_then(|n| n.parse().ok()) {
                    v.push(n);
                }
            }
        }
        v
    };
    let asc = ages(&ui);
    assert!(asc.len() > 3 && asc.windows(2).all(|w| w[0] <= w[1]), "ascending by age: {asc:?}");
    assert_eq!(asc[0], 0);
    click(&mut ui, &sim, &mut cv, centre(age));
    frame(&mut ui, &sim, &cv, Input { time: 3.0, ..Default::default() });
    let desc = ages(&ui);
    assert!(desc.windows(2).all(|w| w[0] >= w[1]), "descending by age: {desc:?}");
    assert_eq!(desc[0], 89);
    // A column that refuses to sort has no click.
    let plain = ui.find("probe:t.head.3").unwrap();
    click(&mut ui, &sim, &mut cv, centre(plain));
    frame(&mut ui, &sim, &cv, Input { time: 4.0, ..Default::default() });
    assert_eq!(ages(&ui)[0], 89, "still descending by age");
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
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
    std::fs::write(&hud, src.replace("\"Wealth %d\"", "\"Riches %d\"")).unwrap();
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
    frame_budget(&mods(), 20..=45);
}

/// A live panel rebuilt every frame beside the shell: still under budget,
/// and the shell's layout is not the price of it.
#[test]
fn whole_ui_fits_the_frame_budget_with_a_live_panel() {
    let dir = scratch_mods("livebudget", &[("live", "", &[("ui/live.luau", LIVE)])]);
    frame_budget(&dir, 100..=125);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A probe mod with a readout rebuilt every frame, at a fixed width.
const LIVE: &str = r#"
ui.define("live:clock", function(view)
    return ui.text({ string.format("%.3f", view.time()), w = 80, id = "live:clock" })
end)
ui.mount("top", "live:clock", { order = 95, refresh = "frame" })
ui.define("live:wide", function(view)
    return ui.row({ ui.text({ string.rep("x", math.floor(view.time())), id = "live:wide" }) })
end)
"#;

#[test]
fn a_frame_tier_panel_changes_every_frame_while_the_shell_layout_is_reused() {
    let dir = scratch_mods("livetier", &[("live", "", &[("ui/live.luau", LIVE)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    for i in 0..3 {
        cv.time = 10.0 + i as f64 / 60.0;
        frame(&mut ui, &sim, &cv, Input { time: cv.time, ..Default::default() });
    }
    let layouts = ui.info.layouts;
    let builds = ui.builds;
    for i in 3..33 {
        cv.time = 10.0 + i as f64 / 60.0;
        frame(&mut ui, &sim, &cv, Input { time: cv.time, ..Default::default() });
        let want = format!("\"{:.3}\"", cv.time);
        assert!(ui.snapshot().contains(&want), "frame {i}: the readout shows {want}\n{}", ui.snapshot());
    }
    assert_eq!(ui.builds - builds, 30, "the live panel rebuilt every frame");
    assert_eq!(ui.info.layouts, layouts, "the shell's layout was reused throughout");
    // The other panels kept their cadence: the clock's text is the same
    // object it was (no rebuild of the top bar every frame is observable
    // only by cost; the layout count above is the proof).
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_size_change_still_relays_out() {
    let dir = scratch_mods(
        "livesize",
        &[(
            "live",
            "",
            &[(
                "ui/live.luau",
                &format!("{LIVE}\nui.mount(\"top\", \"live:wide\", {{ order = 96, refresh = \"frame\" }})\n"),
            )],
        )],
    );
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.time = 10.0;
    frame(&mut ui, &sim, &cv, Input { time: cv.time, ..Default::default() });
    frame(&mut ui, &sim, &cv, Input { time: cv.time, ..Default::default() });
    let layouts = ui.info.layouts;
    let w0 = ui.find("live:wide").unwrap()[2];
    // Same width, other content: no relayout.
    cv.time = 10.5;
    frame(&mut ui, &sim, &cv, Input { time: cv.time, ..Default::default() });
    assert_eq!(ui.info.layouts, layouts, "same size, no relayout");
    // One more x: wider, so the bar is laid out again.
    cv.time = 11.0;
    frame(&mut ui, &sim, &cv, Input { time: cv.time, ..Default::default() });
    assert!(ui.info.layouts > layouts, "a wider readout relays out");
    assert!(ui.find("live:wide").unwrap()[2] > w0);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The same budget with a 30x12 grid on screen: a board's worth of cells
/// costs one node in the tree and one hit, not 360 of each.
#[test]
fn whole_ui_fits_the_frame_budget_with_a_grid_mounted() {
    let dir = scratch_mods("gridbudget", &[("board", "", &[("ui/board.luau", BOARD)])]);
    frame_budget(&dir, 20..=45);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A probe mod with a 30x12 grid on the windows layer.
const BOARD: &str = r#"
ui.define("board:grid", function(view)
    return ui.grid({ id = "board:grid", rows = 30, cols = 12, cell_w = 18, cell_h = 18,
        cell = function(r, c) return { text = tostring((r + c) % 10), bg = (r + c) % 2 == 0 and "surface_raised" or "surface" } end })
end)
ui.mount("windows", "board:grid")
"#;

fn frame_budget(dir: &std::path::Path, builds_expected: std::ops::RangeInclusive<u64>) {
    frame_budget_with(dir, 29, builds_expected);
}

/// The budget scene at 200 pawns, labels on: a crowd of named colonists
/// filling the viewport is the case that made label placement the frame.
#[test]
fn whole_ui_fits_the_frame_budget_at_200_pawns_with_labels_on() {
    frame_budget_with(&mods(), 200, 20..=45);
}

/// How much slower than an idle laptop this process runs right now: a fixed
/// integer workload timed against what that laptop did it in.
fn machine_factor() -> f64 {
    const BASELINE_MS: f64 = 0.45;
    let mut best = f64::MAX;
    for _ in 0..5 {
        let t = std::time::Instant::now();
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        for i in 0..400_000u64 {
            x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(i) ^ (x >> 29);
        }
        std::hint::black_box(x);
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
    }
    println!("  calibration {best:.3} ms");
    (best / BASELINE_MS).max(1.0)
}

fn frame_budget_with(dir: &std::path::Path, pawns: i32, builds_expected: std::ops::RangeInclusive<u64>) {
    // Timing tests run one at a time: two of them sharing cores would
    // measure each other.
    static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let mut sim = sim_at(dir);
    let human = sim.world.defs.creature_id("human").unwrap();
    let c = sim.world.colony_center().unwrap();
    let cols = ((pawns as f32).sqrt().ceil() as i32).max(1);
    for i in 0..pawns {
        let p = c.offset(i % cols - cols / 2, i / cols - cols / 2);
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
    // A live panel rebuilds every frame, so there may be no paint-only frame.
    let m_plain = if plain.is_empty() { f64::NAN } else { median(&mut plain) };
    let (m_all, m_build) = (median(&mut all), median(&mut rebuilds));
    // Rebuild frames are few and the test binary runs its other tests on
    // the same cores: their fastest one is the machine's cost, the median
    // is the contention's. A regression raises both.
    let fastest_build = rebuilds.first().copied().unwrap_or(m_build);
    let built = ui.builds - builds_before;
    println!(
        "whole UI, {} nodes, {pawns} pawns: median frame {m_all:.3} ms (paint-only {m_plain:.3} ms, rebuild {m_build:.3} ms); {built} rebuilds in {n} frames",
        ui.info.nodes
    );
    println!("  luau time by mod: {:?}", ui.vm.mod_time);
    assert!(builds_expected.contains(&built), "expected {builds_expected:?} rebuilds in 2 s, got {built}");
    // Shared CI runners are 2-3x slower than a laptop and noisy with it: the
    // same binary measured 0.9 ms locally and 2.0-2.4 ms on CI. The budgets
    // are for a player's machine, so CI gets slack that still catches a real
    // regression, and the rebuild rate above is asserted exactly everywhere.
    // The Windows runner measured 6x a laptop on rebuild-heavy frames
    // (8.5 ms against 1.4 ms for the same binary), so it gets twice the
    // slack of the others.
    // The other tests in this binary run on the same cores, so the budget
    // is also scaled by how fast the machine is right now, against an idle
    // laptop: a regression in the UI's own work still shows.
    let factor = machine_factor();
    println!("  machine factor {factor:.2}");
    let slack = match (std::env::var_os("CI").is_some(), cfg!(windows)) {
        (false, _) => 1.0,
        (true, false) => 3.0,
        (true, true) => 6.0,
    } * factor;
    assert!(m_all < 1.0 * slack, "median frame {m_all:.3} ms (budget {:.1} ms)", 1.0 * slack);
    assert!(
        fastest_build < 2.0 * slack,
        "fastest rebuild frame {fastest_build:.3} ms, median {m_build:.3} ms (budget {:.1} ms)",
        2.0 * slack
    );
}

#[test]
fn a_grid_is_one_node_however_many_cells() {
    let dir = scratch_mods("gridnodes", &[("board", "", &[("ui/board.luau", BOARD)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let snap = ui.snapshot();
    let tree = snap.split("== board:grid").nth(1).expect("the board is built");
    let tree = tree.split("== ").next().unwrap();
    assert_eq!(
        tree.trim().lines().count(),
        1,
        "the grid is one node:
{tree}"
    );
    assert!(tree.contains("grid 30x12 #board:grid"), "{tree}");
    let rect = ui.find("board:grid").expect("laid out");
    assert_eq!((rect[2], rect[3]), (12.0 * 18.0, 30.0 * 18.0), "sized from its cells");
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dragging_across_a_grid_paints_each_cell_once_in_order() {
    let dir = scratch_mods(
        "griddrag",
        &[(
            "paint",
            "",
            &[(
                "ui/paint.luau",
                r#"
ui.define("paint:board", function(view)
    return ui.col({ id = "paint:panel", bg = "surface", pad = 0, gap = 0, w = 240, align = "start",
        ui.grid({ id = "paint:grid", rows = 4, cols = 4, cell_w = 20, cell_h = 20, gap = 0,
            cell = function(r, c) return tostring(r) .. tostring(c) end,
            on_press = function(r, c) return "v" .. r .. c end,
            on_paint = function(r, c, value)
                ui.set_state("paint:log", ui.state("paint:log", "") .. r .. c .. "=" .. value .. " ")
            end }),
        ui.text({ ui.state("paint:log", "-"), id = "paint:log" }),
    })
end)
ui.mount("windows", "paint:board")
"#,
            )],
        )],
    );
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let g = ui.find("paint:grid").expect("the grid is laid out");
    let at = |r: f32, c: f32| (g[0] + (c - 0.5) * 20.0, g[1] + (r - 0.5) * 20.0);
    // Press on (2,2), drag right two cells, back over one, then down.
    frame(&mut ui, &sim, &cv, Input { mouse: at(2.0, 2.0), ..Default::default() });
    frame(&mut ui, &sim, &cv, Input { mouse: at(2.0, 2.0), left_pressed: true, time: 1.0, ..Default::default() });
    for (i, (r, c)) in [(2.0, 3.0), (2.0, 4.0), (2.0, 3.0), (3.0, 3.0)].into_iter().enumerate() {
        frame(&mut ui, &sim, &cv, Input { mouse: at(r, c), time: 2.0 + i as f64, ..Default::default() });
    }
    frame(&mut ui, &sim, &cv, Input { mouse: at(3.0, 3.0), left_released: true, time: 9.0, ..Default::default() });
    frame(&mut ui, &sim, &cv, Input { mouse: at(3.0, 3.0), time: 10.0, ..Default::default() });
    // Released: moving on paints nothing more.
    frame(&mut ui, &sim, &cv, Input { mouse: at(4.0, 4.0), time: 11.0, ..Default::default() });
    let snap = ui.snapshot();
    assert!(
        snap.contains("\"22=v22 23=v22 24=v22 33=v22 \""),
        "each cell once, in order, with the press value:\n{snap}"
    );
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_long_list_builds_only_the_rows_on_screen() {
    let dir = scratch_mods(
        "virtuallist",
        &[(
            "lister",
            "",
            &[(
                "ui/list.luau",
                r#"
ui.define("lister:list", function(view)
    return ui.col({ id = "lister:panel", bg = "surface", pad = 0,
        ui.list({ id = "lister:scroll", h = 120, count = 200, row_h = 20,
            row = function(i) return ui.text({ "vrow " .. i, id = "lister:row" .. i }) end }) })
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
    frame(&mut ui, &sim, &cv, Input { time: 1.0, ..Default::default() });
    let built = |ui: &rim_ui::Ui| ui.snapshot().matches("\"vrow ").count();
    assert!(built(&ui) < 40, "{} rows built for a 120 px window", built(&ui));
    assert!(ui.find("lister:row1").is_some() && ui.find("lister:row100").is_none());
    let area = ui.find("lister:scroll").expect("the scroll area is laid out");
    let at = centre(area);
    for i in 0..200 {
        frame(&mut ui, &sim, &cv, Input { mouse: at, wheel: -5.0, time: 2.0 + i as f64, ..Default::default() });
    }
    frame(&mut ui, &sim, &cv, Input { mouse: at, time: 300.0, ..Default::default() });
    let got = ui.scroll_offset("lister:scroll").expect("scroll state");
    assert_eq!(got, 200.0 * 20.0 - 120.0, "scrolled to the end");
    assert!(built(&ui) < 40, "{} rows built at the end", built(&ui));
    let last = ui.find("lister:row200").expect("the last row is built");
    assert!((last[1] + last[3] - got - (area[1] + area[3])).abs() <= 1.0, "the last row sits at the bottom");
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    let _ = std::fs::remove_dir_all(&dir);
}

/// The kit gallery, with its board and long table, renders without a
/// warning: every sample is a real use of the kit.
#[test]
fn the_kit_gallery_renders_clean() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.show_devtools = true;
    frame(&mut ui, &sim, &cv, Default::default());
    let toggle = ui.find("core:devtools.gallery").expect("the gallery toggle");
    click(&mut ui, &sim, &mut cv, centre(toggle));
    frame(&mut ui, &sim, &cv, Input { time: 1.0, ..Default::default() });
    let snap = ui.snapshot();
    assert!(snap.contains("grid 8x12 #core:gallery.grid"), "{snap}");
    assert!(ui.find("core:gallery.table").is_some(), "the table's list is laid out");
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    // Painting the board flips a cell and the next build shows it.
    let g = ui.find("core:gallery.grid").unwrap();
    let at = (g[0] + 5.0, g[1] + 5.0);
    frame(&mut ui, &sim, &cv, Input { mouse: at, left_pressed: true, time: 2.0, ..Default::default() });
    frame(&mut ui, &sim, &cv, Input { mouse: at, left_released: true, time: 3.0, ..Default::default() });
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
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
    cv.stuff = vec![mat("core:wood", "wood", 12, true), mat("core:stone", "stone blocks", 0, false)];
    frame(&mut ui, &sim, &cv, Default::default());
    let snap = ui.snapshot();
    assert!(ui.find("core:toolbar.buttons").is_some(), "the toolbar must still build:\n{snap}");
    assert!(ui.find("core:stuff").is_some(), "the material row is there:\n{snap}");
    assert!(
        ui.find("core:stuff.core:wood").is_some() && ui.find("core:stuff.core:stone").is_some(),
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

/// Every user-visible string in core's UI goes through `ui.t`, so a language
/// file can back it later without a hunt. The static scan is the criterion;
/// the runtime set is what a frame actually asked for.
#[test]
fn strings_go_through_one_door() {
    let dir = mods().join("core").join("ui");
    // Every `t("<key>", ...)` call, by hand: a regex crate is not worth it.
    let mut keys = Vec::new();
    for f in ["hud.luau", "devtools.luau", "labels.luau", "keys.luau", "window.luau"] {
        let src = std::fs::read_to_string(dir.join(f)).unwrap();
        let mut rest = src.as_str();
        while let Some(i) = rest.find("t(\"") {
            // `slot("core:x")` and `mount("top", ...)` also end in `t("`:
            // a real call has nothing identifier-like before the `t`.
            let boundary = rest[..i].chars().next_back().is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '.'));
            let after = &rest[i + 3..];
            if !boundary {
                rest = after;
                continue;
            }
            let Some(end) = after.find('"') else { break };
            let key = &after[..end];
            let ok = key.contains(':')
                && key.contains('.')
                && key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | ':' | '.'));
            assert!(ok, "{f}: key {key:?} is not <mod>:<part>.<name>");
            keys.push(key.to_string());
            rest = &after[end..];
        }
    }
    assert!(keys.len() >= 40, "core wraps its strings: {} keys", keys.len());
    let mut uniq = keys.clone();
    uniq.sort();
    uniq.dedup();
    // A key may be reused on purpose (the stuff stats appear twice); every
    // key names its mod and its part.
    assert!(uniq.iter().all(|k| k.starts_with("core:") && k.contains('.')), "namespaced, dotted: {uniq:?}");
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let used = ui.vm.string_keys();
    assert!(!used.is_empty(), "a frame asks for strings");
    for k in &used {
        assert!(uniq.contains(k), "runtime asked for {k}, which the scripts do not declare");
    }
}

/// The door is real: a mod's `ui/lang.toml` overrides a string by key and
/// the frame shows the override, and two mods setting one key is reported.
#[test]
fn a_language_file_overrides_a_string() {
    let dir = scratch_mods(
        "lang",
        &[
            ("deutsch", "", &[("ui/lang.toml", "\"core:topbar.help\" = \"Leertaste Pause\"\n")]),
            ("dansk", "", &[("ui/lang.toml", "\"core:topbar.help\" = \"Mellemrum pause\"\n")]),
        ],
    );
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    let snap = ui.snapshot();
    // Which of the two wins is the loader's ordering, not this test's business.
    let german = snap.contains("Leertaste Pause");
    let danish = snap.contains("Mellemrum pause");
    assert!(german != danish, "exactly one mod's string shows: german={german} danish={danish}\n{snap}");
    assert!(!snap.contains("Space pause"), "the default is gone:\n{snap}");
    assert!(
        ui.warnings().iter().any(|w| w.contains("UI conflict: string 'core:topbar.help'")),
        "two mods setting one key is reported: {:?}",
        ui.warnings()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// No bare user-visible literal in core's UI scripts: anything a player
/// reads is `t(...)`. Ids, tokens and format arguments are not strings a
/// player reads, and the kit takes its text from callers.
#[test]
fn no_bare_literals_in_core_ui() {
    let dir = mods().join("core").join("ui");
    // A quoted literal starting with a letter right after one of these is
    // something a player reads.
    const HEADS: &[&str] = &[
        "label(\"",
        "label = \"",
        "text = \"",
        "tooltip = \"",
        "heading(\"",
        "bubble({ text = \"",
        "toast({ text = \"",
    ];
    let bare = |line: &str| {
        HEADS.iter().any(|h| {
            line.match_indices(h)
                .any(|(i, _)| line[i + h.len()..].chars().next().is_some_and(|c| c.is_ascii_alphabetic()))
        })
    };
    let mut hits = Vec::new();
    for f in ["hud.luau", "devtools.luau", "labels.luau", "keys.luau", "window.luau"] {
        let src = std::fs::read_to_string(dir.join(f)).unwrap();
        for (n, line) in src.lines().enumerate() {
            if line.trim_start().starts_with("--") {
                continue;
            }
            if bare(line) {
                hits.push(format!("{f}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(hits.is_empty(), "bare strings a player would read:\n{}", hits.join("\n"));
}

const KEYLESS_MOD: &str = r#"
ui.bind("probe:one", { label = "Palette only" }, function()
    ui.set_state("probe:fired", ui.state("probe:fired", 0) + 1)
end)
ui.bind("probe:two", { label = "Palette only too" }, function() end)
ui.define("probe:count", function(view)
    return ui.text({ "fired=" .. ui.state("probe:fired", 0), id = "probe:count" })
end)
ui.mount("top", "probe:count", { order = 90 })
"#;

/// A bind with no key is the palette's alone: two of them don't conflict,
/// nothing fires them from the keyboard, and a player can give one a key.
#[test]
fn a_bind_without_a_key_is_palette_only_until_given_one() {
    let dir = scratch_mods("keyless", &[("probe", "", &[("ui/bind.luau", KEYLESS_MOD)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    frame(&mut ui, &sim, &cv, Default::default());
    assert!(
        ui.warnings().iter().all(|w| !w.contains("probe:")),
        "no conflict between keyless binds: {:?}",
        ui.warnings()
    );
    let mut t = 1.0;
    ui.rebind("probe:one", Some("g"));
    press(&mut ui, &sim, &cv, &mut t, "g");
    assert!(ui.snapshot().contains("fired=1"), "the player's key fires it: {}", ui.snapshot());
    press(&mut ui, &sim, &cv, &mut t, "ctrl+k");
    assert!(ui.find("core:palette.probe:two").is_some(), "a keyless bind is in the palette");
    let _ = std::fs::remove_dir_all(&dir);
}
