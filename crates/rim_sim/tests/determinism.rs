use rim_sim::{Command, Sim, TICKS_PER_DAY};
use std::path::Path;

fn run(seed: u64, ticks: u64) -> u64 {
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let mut sim = Sim::new(&mods, seed).expect("mods load");
    let chop = sim.world.defs.lookup("designation", "chop").unwrap();
    let wall = sim.world.defs.thing_id("wall_wood").unwrap();
    let c = sim.world.colony_center().unwrap();
    // Commands must replay identically too.
    sim.push(Command::Designate { designation: chop, a: c.offset(-12, -12), b: c.offset(12, 12) });
    sim.push(Command::Build { thing: wall, a: c.offset(2, 2), b: c.offset(6, 2) });
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
