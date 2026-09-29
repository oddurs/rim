//! Shared setup for UI engine tests: a real simulation, core's real UI.
#![allow(dead_code)]

pub mod raster;

use rim_sim::Sim;
use rim_ui::view::{ClientView, ToolView};
use rim_ui::{Input, Output, Ui};
use std::path::{Path, PathBuf};

/// Whether to assert wall-clock budgets. Off by default: a gating test counts
/// work, and time is checked where nothing else competes for the cores, by
/// CI's isolated budget step (`RIM_BUDGETS=1`, one test at a time). DESIGN.md
/// §8a.
pub fn timing_budgets() -> bool {
    std::env::var_os("RIM_BUDGETS").is_some()
}

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
    // The client's tools, as `toolbar` in rim_client files them: orders,
    // buildables by menu in definition order, and zones.
    let tool = |key: String, label: &str, color: [u8; 3], category: &str, group: &str| ToolView {
        key,
        label: label.into(),
        color,
        category: category.into(),
        group: group.into(),
        ..Default::default()
    };
    let mut tools = vec![ToolView { active: true, ..tool("select".into(), "Select", [128, 128, 128], "", "") }];
    for d in &defs.designations {
        tools.push(tool(format!("designate:{}", d.id), &d.label, d.rgb, "orders", ""));
    }
    tools.push(tool("cancel".into(), "Cancel", [200, 80, 80], "orders", ""));
    let mut menus: Vec<&str> = Vec::new();
    for t in defs.things.iter().filter_map(|t| t.build.as_ref()) {
        if !menus.contains(&t.menu.as_str()) {
            menus.push(&t.menu);
        }
    }
    for menu in menus {
        for t in defs.things.iter().filter(|t| t.build.as_ref().is_some_and(|b| b.menu == menu)) {
            let b = t.build.as_ref().unwrap();
            let cost = if b.free {
                "free".to_string()
            } else {
                b.stuff.as_ref().map_or(String::new(), |s| format!("{} {}", s.count, s.category))
            };
            tools.push(ToolView {
                cost,
                work: b.work,
                hp: t.hp,
                ..tool(format!("build:{}", t.id), &t.label, t.rgb, "build", menu)
            });
        }
    }
    tools.push(tool("stockpile".into(), "Stockpile", [115, 166, 242], "zones", ""));
    tools.push(tool("clear_zone".into(), "Clear zone", [150, 150, 170], "zones", ""));
    ClientView {
        screen: (1600.0, 960.0),
        scale: 1.0,
        cam: (c.x as f32 + 0.5, c.y as f32 + 0.5, 28.0),
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
            format!(
                "id = \"{id}\"\nname = \"{id}\"\nversion = \"0.0.0\"\napi = \"{}.{}\"\ndepends = [\"core\"]\n{manifest_extra}\n",
                rim_sim::API_VERSION.0,
                rim_sim::API_VERSION.1
            ),
        )
        .unwrap();
        for (file, body) in *files {
            let path = m.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
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

/// Put every colonist in core's Hand role, which sets nothing: for board
/// tests that shouldn't depend on Auto's plan.
pub fn hands(s: &mut Sim) {
    let hand =
        s.world.work_roles.iter().position(|r| r.def.as_deref() == Some("core:hand")).expect("core's Hand") as u16;
    for e in s.world.colonists().collect::<Vec<_>>() {
        let mut p = s.world.ecs.get::<&mut rim_sim::world::Pawn>(e).unwrap();
        p.work_role = Some(hand);
        p.plan.clear();
        p.proposal.clear();
    }
}
