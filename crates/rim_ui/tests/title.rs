//! The title screen: the only layer built before a colony, and never in one.

mod common;

use common::*;
use rim_ui::view::{ClientView, SaveView, UiAction};
use rim_ui::Input;

fn centre(r: [f32; 4]) -> (f32, f32) {
    (r[0] + r[2] / 2.0, r[1] + r[3] / 2.0)
}

fn title_screen() -> ClientView {
    let save = |file: &str, day, age, error: Option<&str>| SaveView {
        path: format!("/saves/{file}"),
        file: file.into(),
        day,
        colonists: vec!["Tarn".into(), "Mira".into()],
        age,
        error: error.map(Into::into),
    };
    ClientView {
        screen: (1600.0, 960.0),
        scale: 1.0,
        title: true,
        saves: vec![save("colony-7.rim", 12, 3_600.0, None), save("colony-3.rim", 0, 90_000.0, Some("not a rim save"))],
        ..Default::default()
    }
}

#[test]
fn before_a_colony_only_the_title_screen_is_built() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let mut cv = title_screen();
    frame(&mut ui, &sim, &cv, Input::default());
    assert!(ui.find("core:title.panel").is_some(), "the title screen");
    assert!(ui.find("core:inspector").is_none(), "no game HUD before a game");
    assert!(ui.find("core:toolbar.select").is_none());

    let cont = ui.find("core:title.continue").expect("continue, since there is a save");
    assert_eq!(click(&mut ui, &sim, &mut cv, centre(cont)), vec![UiAction::Load("/saves/colony-7.rim".into())]);
    let new = ui.find("core:title.new").unwrap();
    assert_eq!(click(&mut ui, &sim, &mut cv, centre(new)), vec![UiAction::NewColony]);
    let listed = ui.find("core:title.save.colony-7.rim").unwrap();
    assert_eq!(click(&mut ui, &sim, &mut cv, centre(listed)), vec![UiAction::Load("/saves/colony-7.rim".into())]);
    // A save that can't be read says why, and clicking it does nothing.
    let broken = ui.find("core:title.save.colony-3.rim").unwrap();
    assert_eq!(click(&mut ui, &sim, &mut cv, centre(broken)), vec![]);

    // With no saves, there's nothing to continue.
    cv.saves.clear();
    frame(&mut ui, &sim, &cv, Input::default());
    assert!(ui.find("core:title.continue").is_none());
    assert!(ui.find("core:title.new").is_some());
}

#[test]
fn in_a_game_the_title_screen_is_never_built() {
    let sim = sim_at(&mods());
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.saves = title_screen().saves;
    frame(&mut ui, &sim, &cv, Input::default());
    assert!(ui.find("core:title.panel").is_none());
    assert!(ui.find("core:inspector").is_some());
}
