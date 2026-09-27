//! The grid has levels (DESIGN.md §6d): one map, every level's plane in the
//! same arrays, the surface first. A position without a level is on the
//! surface, and a world without levels saves as it did before.

mod common;

use rim_sim::map::Map;
use rim_sim::path::Goal;
use rim_sim::snapshot::Snapshot;
use rim_sim::{IVec, Sim};

#[test]
fn every_cell_on_every_level_has_one_index() {
    let m = Map::with_levels(10, 8, 3, 2);
    assert_eq!(m.levels(), -3..=2);
    assert_eq!(m.cells(), 10 * 8 * 6);
    for i in 0..m.cells() {
        let p = m.pos(i);
        assert!(m.inb(p), "{p:?}");
        assert_eq!(m.idx(p), i, "{p:?}");
    }
    // Code that only knows the surface indexes the first plane.
    assert_eq!(m.idx(IVec::new(9, 7)), 79);
    assert!(!m.inb(IVec::at(0, 0, -4)) && !m.inb(IVec::at(0, 0, 3)));
}

#[test]
fn chunks_are_per_level_and_the_surface_comes_first() {
    let m = Map::with_levels(64, 64, 1, 0);
    let (cx, cy) = m.chunks();
    assert_eq!(m.level_chunks(0), 0..(cx * cy) as usize);
    let below = m.level_chunks(-1);
    assert_eq!(below.start, (cx * cy) as usize);
    for c in below {
        let o = m.chunk_origin(c);
        assert_eq!(o.z, -1);
        assert_eq!(m.chunk_of(o), c);
    }
}

#[test]
fn a_surface_position_saves_as_it_did_before_levels() {
    #[derive(serde::Serialize)]
    struct Flat {
        x: i32,
        y: i32,
    }
    let now = rmp_serde::to_vec_named(&IVec::new(3, 4)).unwrap();
    assert_eq!(now, rmp_serde::to_vec_named(&Flat { x: 3, y: 4 }).unwrap(), "no z written for the surface");
    let back: IVec = rmp_serde::from_slice(&now).unwrap();
    assert_eq!(back, IVec::at(3, 4, 0));
    let deep: IVec = rmp_serde::from_slice(&rmp_serde::to_vec_named(&IVec::at(3, 4, -2)).unwrap()).unwrap();
    assert_eq!(deep.z, -2);
}

#[test]
fn cells_on_different_levels_never_touch() {
    let (a, b) = (IVec::at(5, 5, 0), IVec::at(5, 5, -1));
    assert!(a.chebyshev(b) > 1_000);
    assert!(!Goal::Touch(b).satisfied(a));
    assert_eq!(a.octile(b), rim_sim::OCTILE_PER_LEVEL, "one level apart, nothing across");
}

/// Open ground on two levels, one under the other: each is its own region,
/// and nothing reaches across without a way between them.
#[test]
fn regions_stay_on_their_level() {
    let mut m = Map::with_levels(20, 20, 1, 0);
    for y in 0..20 {
        for x in 0..20 {
            m.set_terrain(IVec::at(x, y, -1), 0, 100);
        }
    }
    m.ensure_regions();
    let (top, under) = (IVec::new(4, 4), IVec::at(4, 4, -1));
    assert_ne!(m.region_at(top), 0);
    assert_ne!(m.region_at(under), 0);
    assert_ne!(m.region_at(top), m.region_at(under));
    assert!(m.can_reach(under, Goal::Cell(IVec::at(15, 15, -1))));
    assert!(!m.can_reach(top, Goal::Cell(under)));

    // A wall below rebuilds the level below, for each faction, and nothing else.
    let (before, surface) = (m.region_rebuilds, m.region_at(top));
    m.set_terrain(IVec::at(10, 10, -1), 0, 0);
    m.ensure_regions();
    assert_eq!(m.region_rebuilds - before, rim_sim::world::Faction::ALL.len() as u64);
    assert_eq!(m.region_at(top), surface);
}

/// A world with a level below saves and loads to the same bytes, with what
/// stands down there where it was.
#[test]
fn a_world_with_levels_round_trips() {
    let mods = common::mods();
    let mut sim = Sim::build_with(&mods, 3, &|m| m == "core", 64, 1, 0).unwrap();
    let floor = sim.world.defs.lookup("terrain", "core:dirt").unwrap();
    let down = IVec::at(10, 10, -1);
    sim.world.map.set_terrain(down, floor, 100);
    let wall = sim.world.defs.thing_id("wall").unwrap();
    let wood = sim.world.defs.thing_id("wood");
    let e = sim.world.spawn_fixture_of(wall, down, false, wood).unwrap();
    for _ in 0..20 {
        sim.step();
    }
    let first = Snapshot::capture(&sim);
    let loaded = first.restore(&mods, &|m| m == "core").unwrap();
    assert_eq!(loaded.world.map.levels(), -1..=0);
    assert_eq!(loaded.world.map.terrain[loaded.world.map.idx(down)], floor);
    assert_eq!(loaded.world.thing(e).map(|t| t.pos), Some(down), "the wall is still down there");
    assert_eq!(loaded.world.map.fixture_at(down), Some(e));
    assert!(Snapshot::capture(&loaded) == first, "save, load, save gives the same snapshot");
}

/// A room dug into rock below the surface is roofed by the rock around it,
/// as one on the surface is by its walls: cover spreads on every level.
#[test]
fn a_room_dug_below_is_roofed_by_its_rock() {
    let mods = common::mods();
    let mut sim = Sim::build_with(&mods, 3, &|m| m == "core", 64, 1, 0).unwrap();
    let (rock, floor) = {
        let d = &sim.world.defs;
        (d.lookup("terrain", "core:granite").unwrap(), d.lookup("terrain", "core:rock_floor").unwrap())
    };
    let m = &mut sim.world.map;
    for y in 0..64 {
        for x in 0..64 {
            m.set_terrain(IVec::at(x, y, -1), rock, 0);
        }
    }
    for y in 20..24 {
        for x in 20..24 {
            m.set_terrain(IVec::at(x, y, -1), floor, 100);
        }
    }
    m.ensure_rooms();
    let room = m.room_at(IVec::at(21, 21, -1)).expect("a room");
    assert_eq!(room.cells, 16);
    assert_eq!(room.uncovered, 0, "the rock around it holds the roof");
    assert!(m.covered(m.idx(IVec::at(22, 22, -1))));
}
