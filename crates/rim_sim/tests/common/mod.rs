//! Helpers for tests that need their own mods folder.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

pub fn mods() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

pub fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else {
            fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

/// A temporary mods folder with copies of the shipped mods in `ship` plus
/// test mods: (id, [(relative path, contents)]). Each test mod depends on
/// everything in `ship`.
pub fn test_mods(name: &str, ship: &[&str], extra: &[(&str, &[(&str, &str)])]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rim-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    for m in ship {
        copy_dir(&mods().join(m), &dir.join(m));
    }
    let deps = ship.iter().map(|m| format!("\"{m}\"")).collect::<Vec<_>>().join(", ");
    for (id, files) in extra {
        let m = dir.join(id);
        fs::create_dir_all(&m).unwrap();
        fs::write(
            m.join("mod.toml"),
            format!("id = \"{id}\"\nname = \"{id}\"\nversion = \"0.1.0\"\napi = \"0.6\"\ndepends = [{deps}]\n"),
        )
        .unwrap();
        for (path, text) in *files {
            let p = m.join(path);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, text).unwrap();
        }
    }
    dir
}

/// Move every entity into a fresh hecs world, last spawned first, keeping
/// its id. This is the worst a load can do to hecs's internal order.
pub fn respawn_reversed(sim: &mut rim_sim::Sim) {
    let mut old = std::mem::take(&mut sim.world.ecs);
    let mut ids: Vec<rim_sim::hecs::Entity> = old.iter().map(|e| e.entity()).collect();
    ids.sort_by_key(|e| std::cmp::Reverse(e.id()));
    let mut new = rim_sim::hecs::World::new();
    for e in ids {
        let taken = old.take(e).expect("live entity");
        new.spawn_at(e, taken);
    }
    sim.world.ecs = new;
}
