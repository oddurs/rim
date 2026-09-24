//! The cross-machine determinism scenario (0157): every platform in CI runs
//! this and must print exactly the same lines.
//!
//!   cargo run --release -p rim_sim --example crosscheck [-- --days 20]
//!
//! All shipped mods, a fixed seed, and a player's commands (chop, forage,
//! a walled hut with a door, a bed and a campfire), so it exercises the
//! engine, the storyteller and its incidents, the weather plugin and the
//! building code. Prints the state hash at the end of each day, so when two
//! platforms disagree the output shows the first day they diverged.

use rim_sim::{Command, IVec, Sim, TICKS_PER_DAY};
use std::path::Path;

fn arg(name: &str, default: u64) -> u64 {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn main() {
    let days = arg("--days", 20);
    let seed = arg("--seed", 1);
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let mut s = Sim::new(&mods, seed).expect("mods load");
    let defs = s.world.defs.clone();
    let des = |id: &str| defs.lookup("designation", id).expect(id);
    let thing = |id: &str| defs.thing_id(id).expect(id);
    let wood = Some(thing("wood"));
    let c = s.world.colony_center().expect("a colony");

    s.push(Command::Designate { designation: des("chop"), a: c.offset(-14, -14), b: c.offset(14, 14) });
    s.push(Command::Designate { designation: des("harvest"), a: c.offset(-20, -20), b: c.offset(20, 20) });
    // A 5x5 hut: walls, a door on the south side, a bed and a campfire.
    let o = c.offset(3, 3);
    for y in 0..5 {
        for x in 0..5 {
            let p = o.offset(x, y);
            let edge = x == 0 || y == 0 || x == 4 || y == 4;
            if edge && p != o.offset(2, 4) {
                s.push(Command::Build { thing: thing("wall"), stuff: wood, a: p, b: p });
            }
        }
    }
    let at = |dx: i32, dy: i32| -> IVec { o.offset(dx, dy) };
    s.push(Command::Build { thing: thing("door"), stuff: wood, a: at(2, 4), b: at(2, 4) });
    s.push(Command::Build { thing: thing("bed"), stuff: wood, a: at(2, 2), b: at(2, 2) });
    s.push(Command::Build { thing: thing("campfire"), stuff: None, a: at(1, 1), b: at(1, 1) });

    println!(
        "crosscheck seed {seed}, {days} days, mods: {}",
        s.mods.iter().map(|m| m.id.as_str()).collect::<Vec<_>>().join(", ")
    );
    for day in 1..=days {
        for _ in 0..TICKS_PER_DAY {
            s.step();
        }
        let w = &s.world;
        println!(
            "day {day:>3}  hash {:016x}  pawns {:>3}  messages {:>4}",
            w.state_hash(),
            w.pawns.len(),
            w.messages.len()
        );
    }
}
