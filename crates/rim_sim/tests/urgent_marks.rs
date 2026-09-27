//! Urgent marks (DESIGN.md §4d): one job, a level sooner than its work
//! type for everyone, winning ties in its level, gone when the job is.

mod common;

use rim_sim::world::{Job, Pawn};
use rim_sim::{Command, Sim};

/// A castaway in Hand (no plan), Harvest pinned at Soon, a wall planned
/// with wood to hand and berries marked nearby. Build stays at `build`.
fn scene(build: u8) -> (Sim, rim_sim::hecs::Entity, rim_sim::hecs::Entity) {
    let mut s = Sim::with_mods(&common::mods(), 2, &|m| m == "core").unwrap();
    let pawn = s.world.colonists().next().unwrap();
    let defs = s.world.defs.clone();
    let hand = s.world.work_roles.iter().position(|r| r.def.as_deref() == Some("core:hand")).unwrap() as u16;
    s.push(Command::AssignWorkRole { pawn, role: hand });
    let work = |id: &str| defs.lookup("work_type", id).unwrap();
    s.push(Command::SetPriority { pawn, work: work("core:harvest"), level: 2 });
    s.push(Command::SetPriority { pawn, work: work("core:build"), level: build });
    let c = s.world.pawn_pos(pawn).unwrap();
    let (wall, wood) = (defs.thing_id("wall").unwrap(), defs.thing_id("wood").unwrap());
    s.world.place_item(wood, c.offset(1, 1), 20);
    let spot = common::loose_cells(&s, c.offset(3, 0), 1)[0];
    s.push(Command::Build { thing: wall, stuff: Some(wood), a: spot, b: spot });
    let harvest = defs.lookup("designation", "harvest").unwrap();
    s.push(Command::Designate { designation: harvest, a: c.offset(-25, -25), b: c.offset(25, 25) });
    s.step();
    let bp = s
        .world
        .ecs
        .query::<(hecs::Entity, &rim_sim::world::Blueprint)>()
        .iter()
        .map(|(e, _)| e)
        .next()
        .expect("the wall's blueprint");
    (s, pawn, bp)
}

/// The first job of a kind worth telling apart, over up to `ticks`.
fn first_job(s: &mut Sim, pawn: rim_sim::hecs::Entity, ticks: u32) -> Option<&'static str> {
    for _ in 0..ticks {
        s.step();
        match s.world.ecs.get::<&Pawn>(pawn).unwrap().job {
            Job::Construct { .. } | Job::Deliver { .. } => return Some("build"),
            Job::Harvest { .. } => return Some("harvest"),
            _ => {}
        }
    }
    None
}

#[test]
fn an_urgent_wall_comes_before_a_soon_harvest() {
    let (mut calm, pawn, _) = scene(3);
    assert_eq!(first_job(&mut calm, pawn, 2000), Some("harvest"), "unmarked, Soon beats Later");

    let (mut s, pawn, bp) = scene(3);
    s.push(Command::MarkUrgent { target: bp, on: true });
    assert_eq!(first_job(&mut s, pawn, 2000), Some("build"), "marked, Later counts as Soon and wins the tie");
}

#[test]
fn the_mark_goes_when_the_wall_stands() {
    let (mut s, pawn, bp) = scene(3);
    s.push(Command::MarkUrgent { target: bp, on: true });
    s.step();
    assert_eq!(s.world.urgent_count(), 1);
    for _ in 0..20_000 {
        s.step();
        if s.world.ecs.get::<&rim_sim::world::Blueprint>(bp).is_err() {
            break;
        }
    }
    assert!(s.world.ecs.get::<&rim_sim::world::Blueprint>(bp).is_err(), "the wall got built");
    s.step();
    assert_eq!(s.world.urgent_count(), 0, "and its mark went with it");
    let _ = pawn;
}

#[test]
fn a_never_stays_never_when_marked() {
    let (mut s, pawn, bp) = scene(0);
    s.push(Command::MarkUrgent { target: bp, on: true });
    assert_eq!(first_job(&mut s, pawn, 3000), Some("harvest"), "Build at never: the mark doesn't lift it");
}

#[test]
fn only_a_job_takes_a_mark() {
    let (mut s, pawn, _) = scene(3);
    s.push(Command::MarkUrgent { target: pawn, on: true });
    s.step();
    assert_eq!(s.world.urgent_count(), 0, "a colonist isn't a job");
}

#[test]
fn marks_are_saved_and_replay_the_same() {
    let run = |save_at: Option<u64>| {
        let (mut s, _, bp) = scene(3);
        s.push(Command::MarkUrgent { target: bp, on: true });
        for t in 0..1500u64 {
            if Some(t) == save_at {
                s = rim_sim::snapshot::Snapshot::capture(&s).restore(&common::mods(), &|m| m == "core").unwrap();
            }
            s.step();
        }
        s.world.state_hash()
    };
    let h = run(None);
    assert_eq!(h, run(None));
    assert_eq!(h, run(Some(40)), "a load with a live mark carries on the same");
}
