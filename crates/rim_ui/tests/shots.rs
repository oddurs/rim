//! Pictures of the UI, for looking at: `cargo test -p rim_ui --test shots -- --ignored`
//! writes PNGs to target/ui-shots. Not asserted; a way to see a panel
//! without a window.

mod common;
use common::raster::Canvas;
use common::*;
use rim_ui::Input;
use std::path::PathBuf;

fn out_dir() -> PathBuf {
    let d = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/ui-shots"));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A dull green ground, like the world behind the HUD.
const GROUND: [f32; 4] = [0.20, 0.26, 0.18, 1.0];

#[test]
#[ignore]
fn shots() {
    let mut sim = sim_at(&mods());
    let human = sim.world.defs.creature_id("human").unwrap();
    let c = sim.world.colony_center().unwrap();
    for i in 0..4 {
        let p = c.offset(i % 2 - 1, i / 2 - 1);
        if sim.world.map.passable(p) {
            sim.world.spawn_pawn(human, rim_sim::world::Faction::Player, p, None);
        }
    }
    // A stockpile that turned stone away, for the zones panel.
    let stone = sim.world.defs.thing_id("stone").unwrap();
    sim.push(rim_sim::Command::Stockpile { a: c.offset(3, 3), b: c.offset(6, 5), zone: None });
    sim.push(rim_sim::Command::ZoneAllow { zone: 1, thing: stone, on: false });
    sim.step();
    let mut ui = ui_for(&sim);
    let mut cv = client(&sim);
    cv.hover_cell = Some(c);
    let (w, h) = (cv.screen.0 as usize, cv.screen.1 as usize);
    let t = std::cell::Cell::new(0.0);
    let tick = || {
        t.set(t.get() + 0.1);
        t.get()
    };
    let shot = |ui: &mut rim_ui::Ui, cv: &rim_ui::view::ClientView, name: &str, crop: Option<[f32; 4]>| {
        frame(ui, &sim, cv, Input { time: tick(), ..Default::default() });
        let out = frame(ui, &sim, cv, Input { time: tick(), ..Default::default() });
        let mut cv_ = Canvas::new(w, h, GROUND);
        cv_.draw(&out.draw, &ui.text.atlas);
        let cv_ = crop.map(|r| cv_.crop(r)).unwrap_or(cv_);
        cv_.write_png(&out_dir().join(format!("{name}.png")));
    };
    shot(&mut ui, &cv, "hud", None);
    shot(&mut ui, &cv, "hud_top", Some([0.0, 0.0, 1600.0, 120.0]));
    shot(&mut ui, &cv, "hud_bottom", Some([0.0, 760.0, 1600.0, 200.0]));
    // The palette.
    frame(&mut ui, &sim, &cv, Input { pressed: vec!["ctrl+k".into()], time: tick(), ..Default::default() });
    shot(&mut ui, &cv, "palette", Some([540.0, 260.0, 520.0, 440.0]));
    ui.close_window("core:palette");
    // Work priorities.
    ui.open_window("core:work");
    shot(&mut ui, &cv, "work", None);
    ui.close_window("core:work");
    ui.open_window("core:zones");
    shot(&mut ui, &cv, "zones", None);
    ui.close_window("core:zones");
    // Devtools and the gallery.
    cv.show_devtools = true;
    shot(&mut ui, &cv, "devtools", None);
    ui.open_window("core:gallery");
    shot(&mut ui, &cv, "gallery", None);
    ui.close_window("core:gallery");
    cv.show_devtools = false;
    cv.show_profiler = true;
    shot(&mut ui, &cv, "profiler", None);
    // The title screen, before a colony.
    let save = |file: &str, day, colonists: &[&str], age, error: Option<&str>| rim_ui::view::SaveView {
        path: file.into(),
        file: file.into(),
        day,
        colonists: colonists.iter().map(|c| c.to_string()).collect(),
        age,
        error: error.map(Into::into),
    };
    let title = rim_ui::view::ClientView {
        screen: cv.screen,
        scale: 1.0,
        title: true,
        saves: vec![
            save("colony-17.rim", 23, &["Tarn", "Mira", "Oskar", "Wen", "Ilse"], 540.0, None),
            save("colony-9.rim", 4, &["Brannoc"], 7_200.0, None),
            save("colony-3.rim", 1, &[], 260_000.0, Some("the snapshot at tick 0 has no section engine:pawn")),
        ],
        ..Default::default()
    };
    shot(&mut ui, &title, "title", None);
}

/// The Work Board at core's 4 levels and at 9 (a one-line patch): twelve
/// colonists, a siege moving cells, a wall waiting to be built.
#[test]
#[ignore]
fn work_board_at_4_and_9_levels() {
    let nine = scratch_mods(
        "board9",
        &[(
            "fine",
            "",
            &[("defs/scale.toml", "[[patch]]\ntarget = \"priority_scale/core:core\"\nset = { levels = 9 }\n")],
        )],
    );
    for (name, dir) in [("work_board_4", mods()), ("work_board_9", nine.clone())] {
        let mut sim = sim_at(&dir);
        let defs = sim.world.defs.clone();
        let human = defs.creature_id("human").unwrap();
        let c = sim.world.colony_center().unwrap();
        for i in 1..12 {
            sim.world.spawn_pawn(human, rim_sim::world::Faction::Player, c.offset(i % 4, i / 4), None);
        }
        let levels = defs.priority_scale.levels;
        for (k, p) in sim.world.colonists().collect::<Vec<_>>().into_iter().enumerate() {
            for (j, w) in (0..defs.work_types.len()).enumerate() {
                let level = ((k + j * 2 + 1) % (levels as usize + 1)) as u8;
                sim.push(rim_sim::Command::SetPriority { pawn: p, work: w as rim_sim::defs::DefId, level });
            }
        }
        let (wall, wood) = (defs.thing_id("wall").unwrap(), defs.thing_id("wood").unwrap());
        sim.push(rim_sim::Command::Build { thing: wall, stuff: Some(wood), a: c.offset(-6, 6), b: c.offset(-2, 6) });
        sim.push(rim_sim::Command::SetStance { stance: defs.lookup("stance", "core:siege").unwrap() });
        sim.step();
        let mut ui = ui_for(&sim);
        let cv = client(&sim);
        ui.open_window("core:work");
        frame(&mut ui, &sim, &cv, Input { time: 1.0, ..Default::default() });
        let out = frame(&mut ui, &sim, &cv, Input { time: 2.0, ..Default::default() });
        let mut canvas = Canvas::new(cv.screen.0 as usize, cv.screen.1 as usize, GROUND);
        canvas.draw(&out.draw, &ui.text.atlas);
        canvas.write_png(&out_dir().join(format!("{name}.png")));
    }
    let _ = std::fs::remove_dir_all(nine);
}
