mod common;

use rim_sim::{Command, Sim, TICKS_PER_DAY};
use std::path::Path;

fn run(seed: u64, ticks: u64) -> u64 {
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let mut sim = Sim::new(&mods, seed).expect("mods load");
    common::arm(&mut sim);
    let chop = sim.world.defs.lookup("designation", "chop").unwrap();
    let wall = sim.world.defs.thing_id("wall").unwrap();
    let wood = sim.world.defs.thing_id("wood").unwrap();
    let c = sim.world.colony_center().unwrap();
    // Commands must replay identically too.
    sim.push(Command::Designate { designation: chop, a: c.offset(-12, -12), b: c.offset(12, 12) });
    sim.push(Command::Build { stuff: Some(wood), thing: wall, a: c.offset(2, 2), b: c.offset(6, 2) });
    let pawn = sim.world.colonists().next().unwrap();
    let chop_first = sim.world.defs.lookup("work_type", "chop").unwrap();
    sim.push(Command::SetPriority { pawn, work: chop_first, level: 1 });
    sim.push(Command::Stockpile { a: c.offset(-4, -4), b: c.offset(-1, -1), zone: None });
    sim.push(Command::ZoneAllow { zone: 1, thing: wood, on: false });
    for _ in 0..ticks {
        sim.step();
    }
    sim.world.state_hash()
}

#[test]
fn same_seed_same_state() {
    let ticks = TICKS_PER_DAY * 3;
    assert_eq!(run(7, ticks), run(7, ticks));
}

#[test]
fn different_seed_different_state() {
    assert_ne!(run(7, 2000), run(8, 2000));
}

/// Orders given while paused apply at once, at the tick boundary `step`
/// would apply them at: the same game, the same log, and they show before
/// any time passes.
#[test]
fn commands_applied_while_paused_are_the_same_game() {
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let orders = |sim: &mut Sim| {
        common::arm(sim);
        let chop = sim.world.defs.lookup("designation", "chop").unwrap();
        let wall = sim.world.defs.thing_id("wall").unwrap();
        let wood = sim.world.defs.thing_id("wood").unwrap();
        let c = sim.world.colony_center().unwrap();
        sim.push(Command::Designate { designation: chop, a: c.offset(-12, -12), b: c.offset(12, 12) });
        sim.push(Command::Build { stuff: Some(wood), thing: wall, a: c.offset(2, 2), b: c.offset(6, 2) });
    };
    let (mut at_step, mut paused) = (Sim::new(&mods, 9).unwrap(), Sim::new(&mods, 9).unwrap());
    at_step.record();
    paused.record();
    for _ in 0..300 {
        at_step.step();
        paused.step();
    }
    orders(&mut at_step);
    orders(&mut paused);
    let tick = paused.world.tick;
    let marked = |s: &Sim| s.world.ecs.query::<&rim_sim::world::Designated>().iter().count();
    let before = marked(&paused);
    paused.apply_pending();
    assert_eq!(paused.world.tick, tick, "no time passed");
    assert!(marked(&paused) > before, "the designation shows while paused");
    for _ in 0..2000 {
        at_step.step();
        paused.step();
    }
    assert_eq!(at_step.world.state_hash(), paused.world.state_hash());
    assert_eq!(format!("{:?}", at_step.applied()), format!("{:?}", paused.applied()), "logged at the same tick");
}
