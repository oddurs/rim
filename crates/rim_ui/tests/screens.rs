//! Screens as sheets: placed between the docked columns, one at a time,
//! and registered by mods the way core registers its own.

mod common;

use common::*;
use rim_ui::Input;

fn overlaps(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

fn frames(ui: &mut rim_ui::Ui, sim: &rim_sim::Sim, cv: &rim_ui::view::ClientView, t: &mut f64, keys: &[&str]) {
    for k in keys {
        *t += 0.1;
        frame(ui, sim, cv, Input { pressed: vec![k.to_string()], time: *t, ..Default::default() });
    }
    for _ in 0..2 {
        *t += 0.1;
        frame(ui, sim, cv, Input { time: *t, ..Default::default() });
    }
}

#[test]
fn a_sheet_floats_clear_of_the_columns_and_one_replaces_another() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    let mut t = 0.0;
    for screen in [(1600.0, 960.0), (1280.0, 720.0)] {
        cv.screen = screen;
        frames(&mut ui, &sim, &cv, &mut t, &[]);
        if !ui.is_open("core:work") {
            frames(&mut ui, &sim, &cv, &mut t, &["p"]);
        }
        frames(&mut ui, &sim, &cv, &mut t, &[]);
        assert!(ui.is_open("core:work"), "P opens Work");
        let work = ui.find("core:work.window").expect("the Work sheet");
        for id in ["core:colonists", "core:inspector", "core:alerts", "core:messages", "core:topbar", "core:dock"] {
            if let Some(r) = ui.find(id) {
                assert!(!overlaps(work, r), "at {screen:?} the sheet covers {id}: {work:?} vs {r:?}");
            }
        }
        let centre = work[0] + work[2] / 2.0;
        assert!((centre - screen.0 / 2.0).abs() < 1.0, "centred: {work:?}");
    }

    // A second sheet closes the first.
    frames(&mut ui, &sim, &cv, &mut t, &["n"]);
    assert!(ui.is_open("core:news") && !ui.is_open("core:work"), "News replaced Work");
    // An ordinary window leaves a sheet open.
    ui.open_window("core:palette");
    frames(&mut ui, &sim, &cv, &mut t, &[]);
    assert!(ui.is_open("core:news") && ui.is_open("core:palette"));

    // Dragging a sheet's title bar leaves it where it is.
    let title = ui.find("core:news.title").unwrap();
    let before = ui.find("core:news.window").unwrap();
    let at = (title[0] + 20.0, title[1] + title[3] / 2.0);
    t += 0.1;
    frame(&mut ui, &sim, &cv, Input { mouse: at, time: t, ..Default::default() });
    t += 0.1;
    frame(&mut ui, &sim, &cv, Input { mouse: at, left_pressed: true, time: t, ..Default::default() });
    t += 0.1;
    frame(&mut ui, &sim, &cv, Input { mouse: (at.0 + 200.0, at.1 + 100.0), time: t, ..Default::default() });
    t += 0.1;
    frame(
        &mut ui,
        &sim,
        &cv,
        Input { mouse: (at.0 + 200.0, at.1 + 100.0), left_released: true, time: t, ..Default::default() },
    );
    frames(&mut ui, &sim, &cv, &mut t, &[]);
    assert_eq!(ui.find("core:news.window").unwrap(), before, "a sheet doesn't move");
}

const SCREEN_MOD: &str = r#"
local screens = require("@core/ui/screens")
ui.window("probe:herds", { title = "Herds", w = 500, h = 300, sheet = true }, function(view)
    return ui.text({ "herds here", id = "probe:herds.body" })
end)
screens.add({ id = "probe:herds", label = "Herds", key = "h" })
"#;

#[test]
fn a_mod_adds_a_screen_with_one_call() {
    let dir = scratch_mods("screens", &[("probe", "", &[("ui/herds.luau", SCREEN_MOD)])]);
    let sim = sim_at(&dir);
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    let mut t = 0.0;
    frames(&mut ui, &sim, &cv, &mut t, &[]);
    assert!(ui.warnings().is_empty(), "{:?}", ui.warnings());
    let button = ui.find("core:screens.probe:herds").expect("a button on the dock");
    let work = ui.find("core:screens.core:work").expect("beside core's");
    assert!((button[1] - work[1]).abs() < 1.0, "on the same bar: {button:?} {work:?}");
    let xs: Vec<f32> = ["core:work", "core:zones", "core:news", "probe:herds"]
        .iter()
        .map(|id| ui.find(&format!("core:screens.{id}")).unwrap()[0])
        .collect();
    assert!(xs.windows(2).all(|w| w[0] < w[1]), "core's in their order, then the mod's: {xs:?}");
    frames(&mut ui, &sim, &cv, &mut t, &["h"]);
    assert!(ui.is_open("probe:herds") && ui.find("probe:herds.body").is_some(), "its key opens it");
    // Its button opens it too, and shows it open; Work then replaces it.
    frames(&mut ui, &sim, &cv, &mut t, &["h"]);
    assert!(!ui.is_open("probe:herds"));
    let actions = click(&mut ui, &sim, &mut cv, centre(button));
    assert!(actions.is_empty(), "{actions:?}");
    frames(&mut ui, &sim, &cv, &mut t, &[]);
    assert!(ui.is_open("probe:herds"), "the button opens it");
    frames(&mut ui, &sim, &cv, &mut t, &["p"]);
    assert!(ui.is_open("core:work") && !ui.is_open("probe:herds"), "one sheet at a time, a mod's too");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Nothing on the sides moves an open sheet: the hover readout appearing,
/// a palette opening, a colonist selected.
#[test]
fn an_open_sheet_stays_put() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.hover_cell = None;
    let mut t = 0.0;
    frames(&mut ui, &sim, &cv, &mut t, &["p"]);
    frames(&mut ui, &sim, &cv, &mut t, &[]);
    let at_rest = ui.find("core:work.window").unwrap();
    cv.hover_cell = sim.world.colony_center();
    frames(&mut ui, &sim, &cv, &mut t, &[]);
    assert!(ui.find("core:hover").is_some(), "the tile readout is up");
    assert_eq!(ui.find("core:work.window").unwrap(), at_rest, "the readout moved the sheet");
    frames(&mut ui, &sim, &cv, &mut t, &["q"]);
    assert!(ui.find("core:dock.palette.orders").is_some(), "a palette is open");
    assert_eq!(ui.find("core:work.window").unwrap(), at_rest, "the palette moved the sheet");
    cv.selected = sim.world.colonists().next();
    frames(&mut ui, &sim, &cv, &mut t, &[]);
    assert_eq!(ui.find("core:work.window").unwrap(), at_rest, "a selection moved the sheet");

    // Escape: the palette first, then the sheet, and the selection stays.
    frames(&mut ui, &sim, &cv, &mut t, &["escape"]);
    assert!(ui.is_open("core:work") && ui.find("core:dock.palette.orders").is_none());
    let out = frame(&mut ui, &sim, &cv, Input { pressed: vec!["escape".into()], time: t + 0.1, ..Default::default() });
    t += 0.1;
    frames(&mut ui, &sim, &cv, &mut t, &[]);
    assert!(!ui.is_open("core:work"), "Escape closes the sheet");
    assert!(!out.actions.contains(&rim_ui::view::UiAction::Select(None)), "and leaves the selection");
}

/// A window opens with a fade and a rise drawn from the laid-out draws:
/// the frames of the animation lay nothing out again.
#[test]
fn opening_a_window_animates_without_layout() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let cv = client(&sim);
    let mut t = 10.0;
    frames(&mut ui, &sim, &cv, &mut t, &[]);
    t += 0.01;
    frame(&mut ui, &sim, &cv, Input { pressed: vec!["p".into()], time: t, ..Default::default() });
    // The next frame builds and lays the sheet out; count from there.
    t += 0.01;
    let first = frame(&mut ui, &sim, &cv, Input { time: t, ..Default::default() });
    let layouts = ui.info.layouts;
    // The sheet's title text, wherever the animation has put it.
    let bar = ui.find("core:work.title").expect("the sheet's title bar");
    let title = |out: &rim_ui::Output| {
        out.draw
            .iter()
            .find_map(|d| match d {
                rim_ui::paint::Draw::Glyphs { quads, color } => quads
                    .first()
                    .filter(|q| {
                        (bar[0]..bar[0] + bar[2]).contains(&q.dst[0])
                            && (bar[1] - 20.0..bar[1] + bar[3] + 20.0).contains(&q.dst[1])
                    })
                    .map(|q| (q.dst[1], color[3])),
                _ => None,
            })
            .expect("the title's glyphs")
    };
    let (y0, a0) = title(&first);
    let mut last = first;
    for _ in 0..12 {
        t += 1.0 / 60.0;
        last = frame(&mut ui, &sim, &cv, Input { time: t, ..Default::default() });
    }
    let (y1, a1) = title(&last);
    assert_eq!(ui.info.layouts, layouts, "the animation laid something out");
    assert!(a0 < a1 && y0 > y1, "early frames are fainter and lower: ({y0}, {a0}) then ({y1}, {a1})");
}
