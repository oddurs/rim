//! Multi-cell things: one entity in every cell of its footprint (DESIGN.md
//! §6a), so pathing and rooms never learn about footprints.

mod common;

use rim_sim::path::Goal;
use rim_sim::snapshot::Snapshot;
use rim_sim::{IVec, Sim};
use std::path::PathBuf;

/// Core with a wall two cells wide.
fn wide_walls(name: &str) -> PathBuf {
    common::test_mods(
        name,
        &["core"],
        &[("wide", &[("defs/wide.toml", "[[patch]]\ntarget = \"thing/core:wall\"\nset = { size = [2, 1] }\n")])],
    )
}

/// A 3x3 patch of open ground near the colony, cleared.
fn open(s: &mut Sim) -> IVec {
    let c = s.world.colony_center().unwrap();
    let o = (3..60)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))))
        .find(|&o| {
            (-1..=3).all(|x| {
                (-1..=1)
                    .all(|y| s.world.map.passable(o.offset(x, y)) || s.world.map.fixture_at(o.offset(x, y)).is_some())
            })
        })
        .expect("open ground");
    for x in -1..=3 {
        for y in -1..=1 {
            if let Some(f) = s.world.map.fixture_at(o.offset(x, y)) {
                s.world.despawn_thing(f);
            }
        }
    }
    o
}

#[test]
fn a_wide_thing_is_one_entity_in_both_cells_and_blocks_both() {
    let dir = wide_walls("wide-one");
    let mut s = Sim::new(&dir, 2).unwrap();
    let (wall, wood) = (s.world.defs.thing_id("wall").unwrap(), s.world.defs.thing_id("wood"));
    let o = open(&mut s);
    let e = s.world.spawn_fixture_of(wall, o, false, wood).expect("placed");
    assert_eq!(s.world.map.fixture_at(o), Some(e));
    assert_eq!(s.world.map.fixture_at(o.offset(1, 0)), Some(e), "the same entity in the second cell");
    assert!(!s.world.map.passable(o) && !s.world.map.passable(o.offset(1, 0)));
    let walls = s.world.ecs.query::<&rim_sim::world::Thing>().iter().filter(|t| t.def == wall).count();
    assert_eq!(walls, 1, "one thing, not two");

    // Nothing else fits where it stands, even half over it.
    assert!(s.world.spawn_fixture_of(wall, o.offset(1, 0), false, wood).is_none());
    assert!(s.world.spawn_fixture_of(wall, o.offset(-1, 0), false, wood).is_none(), "its second cell would be taken");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn taking_it_away_frees_both_cells_and_the_regions_follow() {
    let dir = wide_walls("wide-free");
    let mut s = Sim::new(&dir, 2).unwrap();
    let (wall, wood) = (s.world.defs.thing_id("wall").unwrap(), s.world.defs.thing_id("wood"));
    let o = open(&mut s);
    let e = s.world.spawn_fixture_of(wall, o, false, wood).unwrap();
    s.world.map.ensure_regions();
    let rev = s.world.map.revision;
    s.world.despawn_thing(e);
    assert_eq!(s.world.map.fixture_at(o), None);
    assert_eq!(s.world.map.fixture_at(o.offset(1, 0)), None);
    assert!(s.world.map.passable(o) && s.world.map.passable(o.offset(1, 0)));
    assert!(s.world.map.revision > rev);
    s.world.map.ensure_regions();
    assert!(s.world.map.can_reach(o.offset(0, -1), Goal::Cell(o.offset(1, 0))), "the freed cell is ground again");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_built_blueprint_covers_its_footprint_and_survives_a_load() {
    let dir = wide_walls("wide-build");
    let mut s = Sim::new(&dir, 2).unwrap();
    let (wall, wood) = (s.world.defs.thing_id("wall").unwrap(), s.world.defs.thing_id("wood"));
    let o = open(&mut s);
    let bp = s.world.spawn_fixture_of(wall, o, true, wood).unwrap();
    assert_eq!(s.world.map.fixture_at(o.offset(1, 0)), Some(bp), "the plan holds both cells");
    assert!(s.world.map.passable(o.offset(1, 0)), "a plan doesn't block yet");
    rim_sim::ai::complete_building(&mut s.world, bp);
    assert!(!s.world.map.passable(o) && !s.world.map.passable(o.offset(1, 0)), "built, it blocks both");

    let back = Snapshot::capture(&s).restore(&dir, &|_| true).unwrap();
    assert_eq!(back.world.map.fixture_at(o.offset(1, 0)), Some(bp), "a load puts it back in both cells");
    assert!(!back.world.map.passable(o.offset(1, 0)));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn sizes_are_checked() {
    let bad = |name: &str, body: &str| {
        let dir = common::test_mods(name, &["core"], &[("bad", &[("defs/b.toml", body)])]);
        let e = Sim::new(&dir, 1).err().expect("refused");
        let _ = std::fs::remove_dir_all(dir);
        e
    };
    let e = bad("size-zero", "[[patch]]\ntarget = \"thing/core:wall\"\nset = { size = [0, 1] }\n");
    assert!(e.contains("each 1 to 8"), "{e}");
    let e = bad("size-item", "[[patch]]\ntarget = \"thing/core:wood\"\nset = { size = [2, 1] }\n");
    assert!(e.contains("only fixtures have a size"), "{e}");
}

/// Whoever stands anywhere in a plan's footprint is out of it once it's
/// built, even in the middle of a 3x3.
#[test]
fn nobody_is_left_inside_a_finished_thing() {
    for (name, size, inside) in [("out-2x1", "[2, 1]", (1, 0)), ("out-3x3", "[3, 3]", (1, 1))] {
        let dir = common::test_mods(
            name,
            &["core"],
            &[(
                "wide",
                &[(
                    "defs/w.toml",
                    format!("[[patch]]\ntarget = \"thing/core:wall\"\nset = {{ size = {size} }}\n").as_str(),
                )],
            )],
        );
        let mut s = Sim::new(&dir, 2).unwrap();
        let (wall, wood) = (s.world.defs.thing_id("wall").unwrap(), s.world.defs.thing_id("wood"));
        let o = open(&mut s);
        let bp = s.world.spawn_fixture_of(wall, o, true, wood).expect("placed");
        let pawn = s.world.colonists().next().unwrap();
        let cell = o.offset(inside.0, inside.1);
        s.world.ecs.get::<&mut rim_sim::world::Pawn>(pawn).unwrap().pos = cell;
        rim_sim::ai::complete_building(&mut s.world, bp);
        let now = s.world.pawn_pos(pawn).unwrap();
        assert!(s.world.map.passable(now), "{name}: moved to open ground, not left at {cell:?} ({now:?})");
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// A blocking wide thing walled in on its anchor's side can still be
/// worked from its far side.
#[test]
fn a_wide_thing_is_reached_from_any_side() {
    let dir = wide_walls("wide-reach");
    let mut s = Sim::new(&dir, 2).unwrap();
    let (wall, wood) = (s.world.defs.thing_id("wall").unwrap(), s.world.defs.thing_id("wood"));
    let o = open(&mut s);
    let e = s.world.spawn_fixture_of(wall, o, false, wood).unwrap();
    // Rock all round the anchor, and over and under the second cell: only
    // the far end is open.
    let rock = s.world.defs.thing_id("granite").unwrap();
    for c in [
        o.offset(-1, -1),
        o.offset(-1, 0),
        o.offset(-1, 1),
        o.offset(0, -1),
        o.offset(0, 1),
        o.offset(1, -1),
        o.offset(1, 1),
    ] {
        s.world.spawn_fixture_of(rock, c, false, None);
    }
    s.world.map.ensure_regions();
    let t = s.world.thing(e).unwrap();
    let from = o.offset(3, 0);
    assert!(!s.world.map.can_reach(from, rim_sim::path::Goal::Touch(o)), "the anchor alone is walled in");
    let goal = s.world.reach_goal(&t);
    assert!(s.world.map.can_reach(from, goal), "its far side is open");
    let path = s.world.pf.find(&s.world.map, from, goal, 10_000, rim_sim::world::Faction::Player);
    assert!(path.is_some(), "and a path gets there");
    let _ = std::fs::remove_dir_all(dir);
}
