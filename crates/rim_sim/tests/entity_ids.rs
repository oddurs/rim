//! Entity ids are the world's, not hecs's: a save, the command log and a
//! script can name an entity, and a load must not change what happens next.

mod common;

use common::respawn_reversed;
use rim_sim::{Command, Sim};
use std::path::Path;

fn sim(seed: u64) -> Sim {
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let mut sim = Sim::new(&mods, seed).expect("mods load");
    common::arm(&mut sim);
    let chop = sim.world.defs.lookup("designation", "chop").unwrap();
    let wall = sim.world.defs.thing_id("wall").unwrap();
    let wood = sim.world.defs.thing_id("wood").unwrap();
    let c = sim.world.colony_center().unwrap();
    sim.push(Command::Designate { designation: chop, a: c.offset(-12, -12), b: c.offset(12, 12) });
    sim.push(Command::Build { stuff: Some(wood), thing: wall, a: c.offset(2, 2), b: c.offset(6, 2) });
    sim
}

/// Seeds 3 and 5 diverged within 1,300 ticks of the reorder when AI loops
/// kept the first of equally near candidates in hecs's order.
#[test]
fn hecs_order_does_not_change_the_game() {
    for seed in [3, 5] {
        let mut straight = sim(seed);
        let mut shuffled = sim(seed);
        for t in 0..2000 {
            if t == 100 {
                respawn_reversed(&mut shuffled);
            }
            straight.step();
            shuffled.step();
            assert_eq!(straight.world.state_hash(), shuffled.world.state_hash(), "seed {seed}, tick {t}");
        }
    }
}

#[test]
fn ids_are_handed_out_in_order_and_never_reused() {
    let mut sim = sim(7);
    let wood = sim.world.defs.thing_id("wood").unwrap();
    let c = sim.world.colony_center().unwrap();
    let before = sim.world.ecs.iter().map(|e| e.entity().id()).max().unwrap();
    let a = sim.world.spawn((rim_sim::world::Thing { def: wood, pos: c, count: 1, hp: 100 },));
    assert_eq!(a.id(), before + 1);
    sim.world.ecs.despawn(a).unwrap();
    let b = sim.world.spawn((rim_sim::world::Thing { def: wood, pos: c.offset(1, 0), count: 1, hp: 100 },));
    assert_eq!(b.id(), a.id() + 1);
}
