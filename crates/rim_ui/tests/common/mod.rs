//! Shared setup for UI engine tests: a real simulation, core's real UI.
#![allow(dead_code)]

pub mod raster;

use rim_sim::Sim;
use rim_ui::view::{ClientView, ToolView};
use rim_ui::{Input, Output, Ui};
use std::path::{Path, PathBuf};

pub fn mods() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

pub fn sim_at(dir: &Path) -> Sim {
    Sim::new(dir, 21).expect("mods load")
}

pub fn ui_for(sim: &Sim) -> Ui {
    Ui::new(rim_ui::mods_of(sim), 1.0, 1.0).expect("ui loads")
}

pub fn client(sim: &Sim) -> ClientView {
    let c = sim.world.colony_center().unwrap();
    let defs = &sim.world.defs;
    let mut tools =
        vec![ToolView { key: "select".into(), label: "Select".into(), color: [128, 128, 128], active: true }];
    for d in &defs.designations {
        tools.push(ToolView {
            key: format!("designate:{}", d.id),
            label: d.label.clone(),
            color: d.rgb,
            active: false,
        });
    }
    ClientView {
        screen: (1600.0, 960.0),
        scale: 1.0,
        cam: (c.x as f32 + 0.5, c.y as f32 + 0.5, 28.0),
        mouse: (800.0, 480.0),
        selected: sim.world.colonists().next(),
        speed: 1,
        tools,
        ..Default::default()
    }
}

pub fn frame(ui: &mut Ui, sim: &Sim, cv: &ClientView, input: Input) -> Output {
    ui.frame(&sim.world, cv, &input)
}

/// Move the mouse to `at` and click there (press one frame, release the next).
pub fn click(ui: &mut Ui, sim: &Sim, cv: &mut ClientView, at: (f32, f32)) -> Vec<rim_ui::view::UiAction> {
    cv.mouse = at;
    frame(ui, sim, cv, Input { mouse: at, ..Default::default() });
    let a = frame(ui, sim, cv, Input { mouse: at, left_pressed: true, ..Default::default() });
    let b = frame(ui, sim, cv, Input { mouse: at, left_released: true, ..Default::default() });
    a.actions.into_iter().chain(b.actions).collect()
}

pub fn centre(r: [f32; 4]) -> (f32, f32) {
    (r[0] + r[2] / 2.0, r[1] + r[3] / 2.0)
}

/// A mods directory with core plus extra mods written by the test.
/// (mod id, extra mod.toml lines, files to write as (path, contents)).
pub type ExtraMod<'a> = (&'a str, &'a str, &'a [(&'a str, &'a str)]);

pub fn scratch_mods(name: &str, extra: &[ExtraMod]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rim-ui-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&mods().join("core"), &dir.join("core"));
    for (id, manifest_extra, files) in extra {
        let m = dir.join(id);
        std::fs::create_dir_all(m.join("ui")).unwrap();
        std::fs::write(
            m.join("mod.toml"),
            format!("id = \"{id}\"\nname = \"{id}\"\nversion = \"0.0.0\"\napi = \"0.4\"\ndepends = [\"core\"]\n{manifest_extra}\n"),
        )
        .unwrap();
        for (file, body) in *files {
            std::fs::write(m.join(file), body).unwrap();
        }
    }
    dir
}

pub fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}
