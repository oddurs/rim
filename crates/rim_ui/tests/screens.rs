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
fn a_sheet_sits_between_the_columns_and_one_replaces_another() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.screen = (1280.0, 720.0);
    let mut t = 0.0;
    frames(&mut ui, &sim, &cv, &mut t, &[]);
    frames(&mut ui, &sim, &cv, &mut t, &["p"]);
    assert!(ui.is_open("core:work"), "P opens Work");
    let work = ui.find("core:work.window").expect("the Work sheet");
    for id in ["core:colonists", "core:inspector", "core:alerts", "core:topbar", "core:dock"] {
        if let Some(r) = ui.find(id) {
            assert!(!overlaps(work, r), "the sheet covers {id}: {work:?} vs {r:?}");
        }
    }
    let band = ui.find(rim_ui::CENTER).unwrap();
    assert!(work[0] >= band[0] - 0.5 && work[0] + work[2] <= band[0] + band[2] + 0.5, "{work:?} in {band:?}");
    assert!((work[3] - band[3]).abs() < 1.0, "a sheet takes the band's height: {work:?} vs {band:?}");

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
