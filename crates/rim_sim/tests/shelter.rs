//! Wind shelter: the lee of walls, rock and trees.

mod common;

use rim_sim::map::Map;
use rim_sim::shelter::{exposure, octant};
use rim_sim::{IVec, Sim};

const DOWNWIND: [(i32, i32); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];

#[test]
fn a_wall_shelters_its_lee_in_every_direction_and_fades() {
    let map = Map::new(21, 21);
    let c = IVec::new(10, 10);
    let mut blocks = vec![0u8; 21 * 21];
    blocks[map.idx(c)] = 100;
    for (o, &(dx, dy)) in DOWNWIND.iter().enumerate() {
        let e = exposure(&map, &blocks, o as u8, 6);
        let at = |k: i32| e[map.idx(IVec::new(c.x + dx * k, c.y + dy * k))];
        assert_eq!(at(0), 0, "octant {o}: the wall itself");
        assert!(at(1) < at(3) && at(3) < at(6), "octant {o}: fades downwind: {} {} {}", at(1), at(3), at(6));
        assert_eq!(at(7), 100, "octant {o}: past the lee");
        assert_eq!(at(-1), 100, "octant {o}: upwind is open");
    }
}

#[test]
fn half_a_wall_shelters_half_as_much() {
    let map = Map::new(21, 21);
    let c = IVec::new(10, 10);
    let mut blocks = vec![0u8; 21 * 21];
    blocks[map.idx(c)] = 50;
    let e = exposure(&map, &blocks, 0, 6);
    let lee = 100 - e[map.idx(c.offset(1, 0))];
    let mut full = blocks.clone();
    full[map.idx(c)] = 100;
    let f = exposure(&map, &full, 0, 6);
    let wall_lee = 100 - f[map.idx(c.offset(1, 0))];
    assert!((lee as i32 * 2 - wall_lee as i32).abs() <= 1, "a tree's lee is half a wall's: {lee} vs {wall_lee}");
}

#[test]
fn directions_round_to_octants() {
    assert_eq!(octant(0.0), 0);
    assert_eq!(octant(44.0), 1);
    assert_eq!(octant(90.0), 2);
    assert_eq!(octant(-90.0), 6);
    assert_eq!(octant(359.0), 0);
}

#[test]
fn shelter_is_worked_out_only_when_the_wind_turns_or_a_blocker_changes() {
    let mut s = Sim::new(&common::mods(), 2).unwrap();
    let defs = s.world.defs.clone();
    let dir = defs.lookup("field", "core:wind_dir").unwrap() as usize;
    let f = defs.lookup("field", "core:wind_exposure").unwrap() as usize;
    s.world.fields.set_ambient(dir, Some(0.0));
    s.step();
    let n = s.world.shelter_recomputes;
    for _ in 0..200 {
        s.step();
    }
    assert_eq!(s.world.shelter_recomputes, n, "nothing changed");
    s.world.fields.set_ambient(dir, Some(10.0));
    s.step();
    assert_eq!(s.world.shelter_recomputes, n, "the same octant");
    s.world.fields.set_ambient(dir, Some(90.0));
    s.step();
    assert_eq!(s.world.shelter_recomputes, n + 1, "the wind turned");

    // A wall goes up in the open: its lee is sheltered.
    let c = s.world.colony_center().unwrap();
    let wall = defs.thing_id("wall").unwrap();
    let wood = defs.thing_id("wood");
    let open = (3..40)
        .flat_map(|r| [c.offset(r, 0), c.offset(-r, 0), c.offset(0, r), c.offset(0, -r)])
        .find(|&p| {
            (0..8).all(|k| s.world.map.passable(p.offset(0, k)) && s.world.map.fixture_at(p.offset(0, k)).is_none())
        })
        .expect("open ground");
    let before = s.world.fields.value(&defs, &s.world.map, f, open.offset(0, 2));
    s.world.spawn_fixture_of(wall, open, false, wood);
    s.step();
    assert_eq!(s.world.shelter_recomputes, n + 2, "a blocker changed");
    assert_eq!(s.world.fields.ambient(f), 100.0, "out in the open is fully exposed");
    let after = s.world.fields.value(&defs, &s.world.map, f, open.offset(0, 2));
    assert!(after < before, "the wind blows south, so south of the wall is sheltered: {before} -> {after}");
}

#[test]
fn an_enclosed_room_has_no_wind() {
    let mut s = Sim::new(&common::mods(), 2).unwrap();
    let defs = s.world.defs.clone();
    let f = defs.lookup("field", "core:wind_exposure").unwrap() as usize;
    let wall = defs.thing_id("wall").unwrap();
    let wood = defs.thing_id("wood");
    let c = s.world.colony_center().unwrap();
    let o = (3..40)
        .flat_map(|r| [c.offset(r, 0), c.offset(-r, 0), c.offset(0, r), c.offset(0, -r)])
        .find(|&o| {
            (0..5).all(|x| {
                (0..5).all(|y| s.world.map.passable(o.offset(x, y)) && s.world.map.fixture_at(o.offset(x, y)).is_none())
            })
        })
        .expect("open ground");
    for y in 0..5 {
        for x in 0..5 {
            if x == 0 || y == 0 || x == 4 || y == 4 {
                s.world.spawn_fixture_of(wall, o.offset(x, y), false, wood);
            }
        }
    }
    s.step();
    assert_eq!(s.world.fields.value(&defs, &s.world.map, f, o.offset(2, 2)), 0.0);
}

#[test]
fn a_lee_is_bounded() {
    let dir = common::test_mods(
        "long-lee",
        &["core"],
        &[(
            "gale",
            &[("defs/fields.toml", "[[patch]]\ntarget = \"field/core:wind_exposure\"\nset = { lee = 100000 }\n")],
        )],
    );
    let err = Sim::new(&dir, 1).err().expect("a lee past the bound is refused");
    assert!(err.contains("lee of 1 to 64"), "{err}");
    let _ = std::fs::remove_dir_all(dir);
}
