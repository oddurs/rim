//! Work types and priorities (DESIGN.md §4d): a colonist does the work at
//! its lowest priority level that has something reachable, and never work
//! at 0.

mod common;

use rim_sim::world::{Blueprint, Job, Pawn};
use rim_sim::{Command, Sim};

#[test]
fn work_types_load_from_core_and_a_mod_can_patch_them() {
    let defs = r#"
[[patch]]
target = "priority_scale/core:core"
set = { levels = 9 }

[[patch]]
target = "work_type/core:mine"
set = { priority = 1 }

[[work_type]]
id = "fish"
label = "Fish"
order = 5
"#;
    let dir = common::test_mods("work-patch", &["core"], &[("probe", &[("defs/work.toml", defs)])]);
    let sim = Sim::new(&dir, 1).unwrap_or_else(|e| panic!("loads: {e}"));
    let d = &sim.world.defs;
    assert_eq!(d.priority_scale.levels, 9);
    assert_eq!(d.work_types[d.lookup("work_type", "core:mine").unwrap() as usize].priority, 1);
    let order: Vec<&str> = d.work_order.iter().map(|&w| d.work_types[w as usize].id.as_str()).collect();
    assert_eq!(order.first(), Some(&"probe:fish"), "a mod's work type takes its place by order: {order:?}");
    assert_eq!(d.build_work, d.lookup("work_type", "core:build"));
    let _ = std::fs::remove_dir_all(dir);
}

/// A colony with wood to build with, a row of walls planned and every tree
/// around designated for chopping: both kinds of work are waiting.
fn busy_colony(build: u8, chop: u8) -> (Sim, rim_sim::hecs::Entity) {
    let mut sim = Sim::new(&common::mods(), 2).unwrap();
    let defs = sim.world.defs.clone();
    let pawn = sim.world.colonists().next().unwrap();
    let c = sim.world.pawn_pos(pawn).unwrap();
    let wood = defs.thing_id("wood").unwrap();
    sim.world.place_item(wood, c.offset(1, 1), 60);
    let (bw, cw) = (defs.lookup("work_type", "build").unwrap(), defs.lookup("work_type", "chop").unwrap());
    sim.push(Command::SetPriority { pawn, work: bw, level: build });
    sim.push(Command::SetPriority { pawn, work: cw, level: chop });
    let chop_d = defs.lookup("designation", "chop").unwrap();
    sim.push(Command::Designate { designation: chop_d, a: c.offset(-15, -15), b: c.offset(15, 15) });
    let wall = defs.thing_id("wall").unwrap();
    sim.push(Command::Build { thing: wall, stuff: Some(wood), a: c.offset(-6, 4), b: c.offset(-2, 4) });
    (sim, pawn)
}

/// Every job the colonist started, over `ticks`, with whether any blueprint
/// was still waiting at the time.
fn jobs(sim: &mut Sim, pawn: rim_sim::hecs::Entity, ticks: u32) -> Vec<(Job, bool)> {
    let mut out: Vec<(Job, bool)> = Vec::new();
    let mut last = String::new();
    for _ in 0..ticks {
        sim.step();
        let job = sim.world.ecs.get::<&Pawn>(pawn).unwrap().job.clone();
        let key = format!("{job:?}");
        if key != last {
            let waiting = sim.world.ecs.query::<&Blueprint>().iter().count() > 0;
            out.push((job, waiting));
            last = key;
        }
    }
    out
}

fn is_chop(j: &Job) -> bool {
    matches!(j, Job::Harvest { forced: false, .. })
}

fn is_build(j: &Job) -> bool {
    matches!(j, Job::Deliver { .. } | Job::Construct { .. })
}

#[test]
fn build_first_builds_while_building_waits_and_chop_first_chops() {
    let (mut sim, pawn) = busy_colony(1, 2);
    let seen = jobs(&mut sim, pawn, 4_000);
    assert!(seen.iter().any(|(j, _)| is_build(j)), "it built: {seen:?}");
    assert!(seen.iter().all(|(j, waiting)| !(is_chop(j) && *waiting)), "no chopping while a blueprint waits: {seen:?}");

    let (mut sim, pawn) = busy_colony(2, 1);
    let seen = jobs(&mut sim, pawn, 1_500);
    let first = seen.iter().find(|(j, _)| is_chop(j) || is_build(j)).map(|(j, _)| j.clone());
    assert!(first.as_ref().is_some_and(is_chop), "chop first chops first: {seen:?}");
}

#[test]
fn a_priority_of_zero_is_never_chosen() {
    let (mut sim, pawn) = busy_colony(0, 0);
    let seen = jobs(&mut sim, pawn, 3_000);
    assert!(seen.iter().all(|(j, _)| !is_chop(j) && !is_build(j)), "neither work at 0: {seen:?}");
}

#[test]
fn a_priority_is_clamped_to_the_scale_and_only_colonists_take_one() {
    let (mut sim, pawn) = busy_colony(9, 3);
    let bw = sim.world.defs.lookup("work_type", "build").unwrap();
    let wild = sim.world.pawns.iter().copied().find(|&e| e != pawn).expect("wildlife");
    sim.push(Command::SetPriority { pawn: wild, work: bw, level: 1 });
    sim.step();
    let p = sim.world.ecs.get::<&Pawn>(pawn).unwrap().priority(&sim.world.defs, bw);
    assert_eq!(p, 4, "core has four levels");
    assert!(sim.world.ecs.get::<&Pawn>(wild).unwrap().priorities.is_empty(), "only colonists take priorities");
}

/// A save made with four levels, loaded after a mod shrank the scale to
/// two: a 4 counts as the last level, not as a level below it.
#[test]
fn a_priority_above_a_shrunk_scale_is_the_last_level() {
    let defs = "[[patch]]\ntarget = \"priority_scale/core:core\"\nset = { levels = 2 }\n";
    let dir = common::test_mods("work-shrink", &["core"], &[("probe", &[("defs/scale.toml", defs)])]);
    let sim = Sim::new(&dir, 1).unwrap();
    let pawn = sim.world.colonists().next().unwrap();
    let hunt = sim.world.defs.lookup("work_type", "core:hunt").unwrap();
    sim.world.ecs.get::<&mut Pawn>(pawn).unwrap().set_priority(hunt, 4);
    assert_eq!(sim.world.ecs.get::<&Pawn>(pawn).unwrap().priority(&sim.world.defs, hunt), 2);
    let _ = std::fs::remove_dir_all(dir);
}

/// The text form of a save names work types by id, like every other def.
#[test]
fn a_text_save_names_work_types_by_id() {
    let path = std::env::temp_dir().join(format!("rim-work-text-{}.rim", std::process::id()));
    let dir = std::env::temp_dir().join(format!("rim-work-text-{}", std::process::id()));
    let (mut sim, pawn) = busy_colony(3, 3);
    let mut save = rim_sim::savefile::SaveFile::create(&path, &mut sim).unwrap();
    let hunt = sim.world.defs.lookup("work_type", "core:hunt").unwrap();
    sim.push(Command::SetPriority { pawn, work: hunt, level: 1 });
    for _ in 0..10 {
        sim.step();
    }
    save.snapshot(&mut sim).unwrap();
    drop(save);
    rim_sim::savetext::unpack(&path, &dir).unwrap();
    // The pawns and the log; not the def table, which lists every id.
    let read = |name: &str| -> String {
        walk(&dir)
            .into_iter()
            .filter(|p| p.file_name().is_some_and(|f| f.to_string_lossy() == name))
            .map(|p| std::fs::read_to_string(p).unwrap())
            .collect()
    };
    assert!(read("pawn.json").contains("\"core:hunt\""), "a pawn's priorities name their work type");
    assert!(read("log.json").contains("\"core:hunt\""), "SetPriority names its work type");
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_file(path);
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}
