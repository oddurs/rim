//! Snow and mud slow walking (DESIGN.md §4c): a stock field's `move_cost`
//! becomes extra cost on the map, which pawns walk at and paths weigh,
//! without ever touching the regions.

mod common;

use rim_sim::command::Command;
use rim_sim::path::Goal;
use rim_sim::world::Faction;
use rim_sim::{IVec, Sim};
use std::path::Path;

fn sim() -> Sim {
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let mut s = Sim::with_mods(&mods, 5, &|m| m == "core" || m == "weather").expect("mods load");
    // Cold and dry: snow lies where it is put, neither falling nor melting.
    for (id, v) in [("temperature", -8.0), ("precipitation", 0.0)] {
        let f = s.world.defs.lookup("field", id).unwrap() as usize;
        s.world.fields.set_ambient(f, Some(v));
    }
    s.step();
    s
}

fn snow(s: &mut Sim, p: IVec, cm: f64) {
    let f = s.world.defs.lookup("field", "weather:snow").unwrap() as usize;
    let defs = s.world.defs.clone();
    s.world.fields.set_stock(&defs, &s.world.map, f, p, cm, false).expect("on the surface");
}

/// Grass, bare of anything, over a box.
fn clear(s: &mut Sim, o: IVec, w: i32, h: i32) {
    let grass = s.world.defs.lookup("terrain", "grass").unwrap();
    for y in 0..h {
        for x in 0..w {
            let p = o.offset(x, y);
            if let Some(f) = s.world.map.fixture_at(p) {
                s.world.despawn_thing(f);
            }
            s.world.map.set_terrain(p, grass, 100);
        }
    }
}

/// Ticks for `pawn` to walk to `to`.
fn walk(s: &mut Sim, pawn: rim_sim::hecs::Entity, to: IVec) -> u32 {
    s.push(Command::Order { pawn, cell: to, on: None, pick: Some("move".into()) });
    for t in 1..5_000 {
        s.step();
        if s.world.pawn_pos(pawn) == Some(to) {
            return t;
        }
    }
    panic!("never got there");
}

#[test]
fn pawns_cross_deep_snow_slower() {
    let mut s = sim();
    let pawn = s.world.colonists().next().unwrap();
    let start = s.world.pawn_pos(pawn).unwrap();
    clear(&mut s, start.offset(0, -2), 16, 5);
    let end = start.offset(14, 0);
    let bare = walk(&mut s, pawn, end);
    // 50 cm, two and a half times the cost, far enough round that no way
    // round it is cheaper.
    for y in -20..=20 {
        for x in -20..36 {
            let p = start.offset(x, y);
            if s.world.map.inb(p) {
                snow(&mut s, p, 50.0);
            }
        }
    }
    s.step();
    assert_eq!(s.world.map.extra_cost(s.world.map.idx(start.offset(5, 0))), 150);
    let deep = walk(&mut s, pawn, start);
    println!("14 cells: {bare} ticks bare, {deep} in 50 cm of snow");
    assert!(deep as f64 > 2.0 * bare as f64, "bare {bare}, deep {deep}");
}

#[test]
fn paths_take_a_cleared_route_when_one_is_close() {
    let mut s = sim();
    let o = s.world.colony_center().unwrap().offset(-20, 8);
    clear(&mut s, o, 36, 9);
    // A snowfield with a shovelled lane two cells off the straight line.
    for y in 0..9 {
        for x in 3..33 {
            snow(&mut s, o.offset(x, y), if y == 6 { 0.0 } else { 40.0 });
        }
    }
    s.step();
    let (from, to) = (o.offset(0, 4), o.offset(35, 4));
    let path = s.world.pf.find(&s.world.map, from, Goal::Cell(to), 100_000, Faction::Player).expect("a path");
    let m = &s.world.map;
    let in_snow = path.iter().filter(|&&p| m.extra_cost(m.idx(p)) > 0).count();
    assert!(in_snow <= 4, "{in_snow} of {} steps in the snow: {path:?}", path.len());
    assert!(path.iter().any(|p| p.y == o.y + 6), "it takes the lane");
}

#[test]
fn snow_rebuilds_no_regions() {
    let mut s = sim();
    let (regions, revision) = (s.world.map.region_rebuilds, s.world.map.revision);
    let o = s.world.colony_center().unwrap().offset(-30, -30);
    for y in 0..60 {
        for x in 0..60 {
            let p = o.offset(x, y);
            if s.world.map.inb(p) {
                snow(&mut s, p, 35.0);
            }
        }
    }
    let changes = s.world.fields.move_changes;
    for _ in 0..100 {
        s.step();
    }
    assert!(s.world.fields.move_changes > changes, "the costs moved");
    assert_eq!(s.world.map.region_rebuilds, regions, "no region was rebuilt for snow");
    assert_eq!(s.world.map.revision, revision, "passability didn't change");
}
