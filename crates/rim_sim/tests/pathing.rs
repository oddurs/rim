//! Pawns find their way, however long it is. The regions decide whether a
//! goal can be reached; once they say yes, the search must find the way,
//! or the pawn asks again and again and never walks it.

use rim_sim::world::{Faction, Job, Pawn};
use rim_sim::{IVec, Sim};
use std::path::Path;

const SIZE: i32 = 250;

/// A map that is one corridor: every other row is deep water, each open
/// only at alternate ends, so the far corner is ~31,000 steps away.
fn serpentine() -> Sim {
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let mut s = Sim::build(&mods, 3, &|_| true, SIZE).expect("mods load");
    let grass = s.world.defs.lookup("terrain", "grass").unwrap();
    let water = s.world.defs.lookup("terrain", "deep_water").unwrap();
    for y in 0..SIZE {
        let gap = if y % 4 == 1 { SIZE - 1 } else { 0 };
        for x in 0..SIZE {
            let p = IVec::new(x, y);
            if let Some(f) = s.world.map.fixture_at(p) {
                s.world.despawn_thing(f);
            }
            if y % 2 == 1 && x != gap {
                s.world.map.set_terrain(p, water, 0);
            } else {
                s.world.map.set_terrain(p, grass, 100);
            }
        }
    }
    s
}

#[test]
fn a_long_way_round_is_found_the_first_time() {
    let mut s = serpentine();
    let human = s.world.defs.creature_id("human").unwrap();
    let walker = s.world.spawn_pawn(human, Faction::Player, IVec::new(0, 0), Some("Walker".into()));
    // Everyone else walks off, so the counters below are the walker's.
    for &e in &s.world.pawns.clone() {
        if e != walker {
            s.world.ecs.get::<&mut Pawn>(e).unwrap().left = true;
        }
    }
    s.step();
    let goal = IVec::new(SIZE - 1, SIZE - 2);
    rim_sim::ai::set_job(&mut s.world, walker, Job::MoveTo { to: goal });
    let (searches, failed) = (s.world.pf.searches, s.world.pf.failed);
    for _ in 0..30 {
        s.step();
    }
    let p = s.world.ecs.get::<&Pawn>(walker).unwrap();
    assert!(p.path.len() > 30_000, "on its way, the long way round: {} steps left", p.path.len());
    assert_eq!(s.world.pf.failed, failed, "no search gave up");
    assert_eq!(s.world.pf.searches, searches + 1, "one search found it");
}
