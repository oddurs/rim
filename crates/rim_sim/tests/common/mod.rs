//! Helpers for tests that need their own mods folder.
#![allow(dead_code)]

use rim_sim::hecs::Entity;
use rim_sim::{Command, IVec, Sim};
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

/// With the stone age loaded, an oak waits for an axe and granite for a
/// hammerstone. Hand every colonist both, so a test that chops, mines and
/// builds still does. Without the plugin, nothing changes.
pub fn arm(sim: &mut rim_sim::Sim) {
    let d = &sim.world.defs;
    let tools: Vec<_> =
        ["primitive:hand_axe", "primitive:hammerstone"].iter().filter_map(|id| d.thing_id(id)).collect();
    for c in sim.world.colonists().collect::<Vec<_>>() {
        let Some(at) = sim.world.pawn_pos(c) else { continue };
        for &t in &tools {
            sim.world.place_item(t, at, 1);
        }
    }
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

/// A colony whose only work is hauling unless a test adds more: every
/// other work type is set to 0, and there's an open patch for a stockpile.
pub fn hauling_colony(seed: u64) -> (Sim, Entity, IVec) {
    let mut sim = Sim::new(&mods(), seed).unwrap();
    let pawn = sim.world.colonists().next().unwrap();
    let defs = sim.world.defs.clone();
    for (w, d) in defs.work_types.iter().enumerate() {
        let level = if d.id == "core:haul" { 1 } else { 0 };
        sim.push(Command::SetPriority { pawn, work: w as rim_sim::defs::DefId, level });
    }
    let c = sim.world.pawn_pos(pawn).unwrap();
    // Room for items: open, and nothing standing there (tall grass is
    // passable, but no stack goes under it).
    let open = |s: &Sim, o: IVec| {
        (0..3).all(|x| {
            (0..3).all(|y| {
                let p = o.offset(x, y);
                s.world.map.passable(p) && s.world.map.item_at(p).is_none() && s.world.map.fixture_at(p).is_none()
            })
        })
    };
    let site = (3..40)
        .flat_map(|r| [c.offset(r, 0), c.offset(-r, 0), c.offset(0, r), c.offset(0, -r)])
        .find(|&o| open(&sim, o))
        .expect("open ground");
    (sim, pawn, site)
}

/// The rock cell nearest `from` that a pawn there can work from beside it.
/// Rock is terrain until someone works it (DESIGN.md §6d), so there is no
/// thing to look for until it is marked.
pub fn nearest_rock(w: &rim_sim::world::World, from: rim_sim::IVec) -> Option<rim_sim::IVec> {
    (0..w.map.w * w.map.h)
        .map(|i| w.map.pos(i as usize))
        .filter(|&p| w.solid_at(p).is_some() && w.map.can_reach(from, rim_sim::path::Goal::Touch(p)))
        .min_by_key(|&p| (p.octile(from), w.map.idx(p)))
}

/// The nearest cells to `from` a stack can be dropped on that no stockpile
/// covers: passable, no item, no fixture, no zone. A test's loose stack
/// then stays loose whatever the map puts near the colonist.
pub fn loose_cells(sim: &Sim, from: IVec, n: usize) -> Vec<IVec> {
    let w = &sim.world;
    let free = |p: IVec| {
        w.map.passable(p)
            && w.map.item_at(p).is_none()
            && w.map.fixture_at(p).is_none()
            && w.zones.at(&w.map, p).is_none()
    };
    (1..60)
        .flat_map(|r: i32| {
            (-r..=r).flat_map(move |dy| {
                (-r..=r).filter(move |dx| dx.abs() == r || dy.abs() == r).map(move |dx| from.offset(dx, dy))
            })
        })
        .filter(|&p| free(p))
        .take(n)
        .collect()
}

/// The `n` open cells nearest the colony: passable, with no fixture or
/// item. Ring by ring, each ring's own cells, so no cell comes twice and
/// two builds never share one.
pub fn open_cells(s: &Sim, n: usize) -> Vec<IVec> {
    let c = s.world.colony_center().expect("a colony");
    (1..30)
        .flat_map(|r| {
            (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))).filter(move |p| p.chebyshev(c) == r)
        })
        .filter(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none())
        .take(n)
        .collect()
}

/// Put every colonist in core's Hand role, which sets nothing: for tests of
/// stances, rules and work choice that shouldn't depend on Auto's plan.
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
