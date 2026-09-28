//! Movement classes (DESIGN.md §6d): a creature whose `[[movement]]` has a
//! drop climbs down a pit's side and up the other, where a walker is
//! stopped. Core ships no climbers, so a mod brings one.

mod common;

use rim_sim::path::Goal;
use rim_sim::world::{Faction, Job, Pawn};
use rim_sim::{IVec, Sim};

const GOATS: &str = r##"
[[movement]]
id = "climbing"
label = "climbing"
drop = 1

[[creature]]
id = "goat"
label = "goat"
color = "#c8c0b0"
size = 0.3
speed = 12
max_hp = 40
melee_damage = 3
melee_cooldown = 60
flees = true
market_value = 30
movement = "climbing"
"##;

/// A world with goats, and a trench round a patch of ground: pits whose
/// floors below are dug out.
fn trenched() -> (Sim, std::path::PathBuf, IVec) {
    let dir = common::test_mods("climbers", &["core"], &[("goats", &[("defs/goats.toml", GOATS)])]);
    let mut s = Sim::build(&dir, 3, &|_| true, 64).unwrap();
    let c = s.world.colony_center().unwrap();
    let air = s.world.defs.terrain.iter().position(|t| t.air).unwrap() as rim_sim::defs::DefId;
    for y in -5i32..=5 {
        for x in -5i32..=5 {
            if x.abs().max(y.abs()) != 5 {
                continue;
            }
            let p = c.offset(x, y);
            for e in [s.world.map.fixture_at(p), s.world.map.item_at(p)].into_iter().flatten() {
                s.world.despawn_thing(e);
            }
            s.world.map.set_terrain(p, air, 0);
            let below = IVec::at(p.x, p.y, -1);
            if let Some(leaves) = s.world.solid_at(below).and_then(|r| r.leaves_r) {
                let cost = s.world.defs.terrain[leaves as usize].path_cost;
                s.world.map.set_terrain(below, leaves, cost);
            }
        }
    }
    s.world.map.ensure_regions();
    (s, dir, c)
}

#[test]
fn a_climber_crosses_a_trench_that_stops_a_walker() {
    let (mut s, dir, c) = trenched();
    let (inside, outside) = (c.offset(2, 0), c.offset(9, 0));
    assert!(s.world.map.passable(inside) && s.world.map.passable(outside), "ground either side");
    let m = &s.world.map;
    assert!(!m.can_reach_as(outside, Goal::Cell(inside), Faction::Wild, false), "a walker can't cross");
    assert!(m.can_reach_as(outside, Goal::Cell(inside), Faction::Wild, true), "a climber can");
    let mut pf = rim_sim::path::Pathfinder::default();
    let path = pf.find_as(m, outside, Goal::Cell(inside), 10_000, Faction::Wild, true).expect("a climb");
    assert!(path.iter().any(|p| p.z == -1), "down into the trench and up: {path:?}");

    // A goat sent inside gets there.
    let goat = s.world.defs.creature_id("goats:goat").unwrap();
    let e = s.world.spawn_pawn(goat, Faction::Wild, outside, None);
    rim_sim::ai::set_job(&mut s.world, e, Job::MoveTo { to: inside });
    let there = (0..4_000).any(|_| {
        s.step();
        s.world.ecs.get::<&Pawn>(e).is_ok_and(|p| p.pos.chebyshev(inside) <= 1 && p.pos.z == 0)
    });
    assert!(there, "the goat climbed across");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn without_climbers_nothing_is_kept_for_them() {
    let mut s = Sim::build(&common::mods(), 3, &|m| m == "core", 64).unwrap();
    let c = s.world.colony_center().unwrap();
    s.world.map.ensure_regions();
    // No creature loaded climbs: asking as a climber is asking as a walker.
    let far = c.offset(10, 10);
    let (walk, climb) = (
        s.world.map.can_reach_as(c, Goal::Cell(far), Faction::Player, false),
        s.world.map.can_reach_as(c, Goal::Cell(far), Faction::Player, true),
    );
    assert_eq!(walk, climb);
}
