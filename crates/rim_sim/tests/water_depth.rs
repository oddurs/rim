//! What depth does (DESIGN.md §6d): wading costs, deep water takes the
//! footing away from whoever can't swim, colonists leave rising water, a
//! sealed door keeps it out.

mod common;

use rim_sim::defs::DefId;
use rim_sim::path::Goal;
use rim_sim::world::{Faction, Pawn};
use rim_sim::{IVec, Sim};

fn core() -> Sim {
    Sim::build(&common::mods(), 3, &|m| m == "core", 64).expect("core loads")
}

/// A door that people pass and water doesn't.
const SEALED: &str = r##"
[[thing]]
id = "sealed_door"
label = "sealed door"
color = "#607080"
category = "building"
door = true
holds_water = true
path_cost = 60
hp = 200
"##;

fn terrain(s: &Sim, id: &str) -> DefId {
    s.world.defs.terrain.iter().position(|t| t.id == format!("core:{id}")).unwrap_or_else(|| panic!("{id}")) as DefId
}

fn set(s: &mut Sim, p: IVec, id: &str) {
    let t = terrain(s, id);
    let cost = s.world.defs.terrain[t as usize].path_cost;
    s.world.map.set_terrain(p, t, cost);
}

fn dig(s: &mut Sim, p: IVec) {
    if let Some(leaves) = s.world.solid_at(p).and_then(|r| r.leaves_r) {
        let cost = s.world.defs.terrain[leaves as usize].path_cost;
        s.world.map.set_terrain(p, leaves, cost);
    }
}

fn run(s: &mut Sim, ticks: u32, done: impl Fn(&Sim) -> bool) -> bool {
    (0..ticks).any(|_| {
        s.step();
        done(s)
    })
}

/// Wet limestone round every open cell of `cells`' ring: what floods them.
fn aquifer_around(s: &mut Sim, cells: &[IVec]) {
    for &c in cells {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let q = c.offset(dx, dy);
            if !cells.contains(&q) && s.world.solid_at(q).is_some() {
                set(s, q, "wet_limestone");
            }
        }
    }
}

#[test]
fn a_flooded_tunnel_takes_the_footing_from_whoever_cant_swim() {
    let dir = common::test_mods("water-tunnel", &["core"], &[("seal", &[("defs/seal.toml", SEALED)])]);
    let mut s = Sim::build(&dir, 3, &|_| true, 64).unwrap();
    let c = s.world.colony_center().unwrap();
    // A dry cell, a sealed door, the tunnel, a sealed door, a dry cell:
    // people pass the doors and water doesn't.
    let row: Vec<IVec> = (0..14).map(|x| IVec::at(c.x - 7 + x, c.y, -1)).collect();
    for &p in &row {
        dig(&mut s, p);
    }
    let sealed = s.world.defs.thing_id("seal:sealed_door").unwrap();
    for &d in [row[1], row[12]].iter() {
        s.world.spawn_fixture_of(sealed, d, false, None).expect("a door");
    }
    let tunnel = &row[2..12];
    s.world.map.ensure_regions();
    let (a, b) = (row[0], row[13]);
    assert!(s.world.map.can_reach_for(a, Goal::Cell(b), Faction::Player), "dry, it is walked end to end");
    aquifer_around(&mut s, tunnel);
    let swim = s.world.defs.fluids[0].swim;
    assert!(run(&mut s, 40_000, |s| s.world.water_depth(tunnel[5]) >= swim), "the tunnel floods past wading");
    for _ in 0..rim_sim::world::WATER_EVERY {
        s.step();
    }
    s.world.map.ensure_regions();
    assert_eq!(s.world.water_depth(a), 0, "the doors keep the ends dry");
    assert!(!s.world.map.passable(tunnel[5]), "deep water is no footing");
    assert!(!s.world.map.can_reach_for(a, Goal::Cell(b), Faction::Player), "and nobody walks the tunnel now");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn colonists_leave_rising_water_and_nobody_drowns() {
    let mut s = core();
    let founder = s.world.colonists().next().unwrap();
    // A room dug at -1 under open ground, stairs down into it, and an
    // aquifer round it.
    let c = s.world.ecs.get::<&Pawn>(founder).unwrap().pos;
    let top = (2..20)
        .flat_map(|r| [c.offset(r, 0), c.offset(-r, 0), c.offset(0, r), c.offset(0, -r)])
        .find(|&p| s.world.can_dig(p))
        .expect("somewhere to dig");
    let mid = IVec::at(top.x, top.y, -1);
    let room: Vec<IVec> = (-2..=2).flat_map(|y| (-2..=2).map(move |x| mid.offset(x, y))).collect();
    for &p in &room {
        dig(&mut s, p);
    }
    let stairs = s.world.defs.thing_id("stairs").unwrap();
    let e = s.world.spawn_fixture_of(stairs, top, false, None).expect("stairs");
    s.world.open_portal(e);
    let human = s.world.defs.start.as_ref().unwrap().creature_r;
    let down = s.world.spawn_pawn(human, Faction::Player, mid.offset(1, 1), None);
    // Drafted, so they stay put until the water moves them.
    s.push(rim_sim::Command::Draft { pawn: down, on: true });
    aquifer_around(&mut s, &room);
    let f = s.world.defs.fluids[0].clone();
    let mut deepest = 0;
    let mut fled = false;
    for _ in 0..20_000u32 {
        s.step();
        let p = s.world.ecs.get::<&Pawn>(down).unwrap().clone();
        deepest = deepest.max(s.world.water_depth(p.pos));
        fled |= matches!(p.job, rim_sim::world::Job::Flee { .. });
    }
    assert!(s.world.water_depth(mid) >= f.swim, "the room flooded");
    let p = s.world.ecs.get::<&Pawn>(down).unwrap().clone();
    assert!(fled, "the rising water sent them out");
    assert!(!p.dead && deepest < f.no_air, "nobody was caught with no air (deepest {deepest})");
    assert!(s.world.water_depth(p.pos) < f.wade, "and they stayed out (at {:?})", p.pos);
}

#[test]
fn a_door_that_holds_water_keeps_it_out() {
    let dir = common::test_mods("water-sealed", &["core"], &[("seal", &[("defs/seal.toml", SEALED)])]);
    let mut s = Sim::build(&dir, 3, &|_| true, 64).unwrap();
    let c = s.world.colony_center().unwrap();
    // Two rooms at -1 in a row with a doorway between: water on one side.
    let wet: Vec<IVec> = (0..3).flat_map(|y| (0..3).map(move |x| IVec::at(c.x + x, c.y + y, -1))).collect();
    let dry: Vec<IVec> = (0..3).flat_map(|y| (0..3).map(move |x| IVec::at(c.x + 4 + x, c.y + y, -1))).collect();
    let door = IVec::at(c.x + 3, c.y + 1, -1);
    for &p in wet.iter().chain(&dry).chain([&door]) {
        dig(&mut s, p);
    }
    let sealed = s.world.defs.thing_id("seal:sealed_door").unwrap();
    s.world.spawn_fixture_of(sealed, door, false, None).expect("the door");
    let mut ring = wet.clone();
    ring.push(door);
    ring.extend(&dry);
    for &p in &wet {
        for (dx, dy) in [(-1, 0), (0, 1), (0, -1)] {
            let q = p.offset(dx, dy);
            if !ring.contains(&q) && s.world.solid_at(q).is_some() {
                set(&mut s, q, "wet_limestone");
            }
        }
    }
    let full = rim_sim::water::FULL;
    assert!(run(&mut s, 40_000, |s| wet.iter().all(|&p| s.world.water_depth(p) == full)), "one side fills");
    assert!(dry.iter().all(|&p| s.world.water_depth(p) == 0), "the other stays dry");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_miner_who_breaches_a_lake_at_the_end_of_a_tunnel_gets_out() {
    let mut s = core();
    let c = s.world.colony_center().unwrap();
    // Stairs down on open ground, and a tunnel at -1 running east from
    // their foot.
    let top = (2..20)
        .flat_map(|r| [c.offset(r, 0), c.offset(-r, 0), c.offset(0, r), c.offset(0, -r)])
        .find(|&p| s.world.can_dig(p) && (1..12).all(|x| s.world.solid_at(IVec::at(p.x + x, p.y, -1)).is_some()))
        .expect("somewhere to dig");
    let tunnel: Vec<IVec> = (0..12).map(|x| IVec::at(top.x + x, top.y, -1)).collect();
    for &p in &tunnel {
        dig(&mut s, p);
    }
    let stairs = s.world.defs.thing_id("stairs").unwrap();
    let e = s.world.spawn_fixture_of(stairs, top, false, None).expect("stairs");
    s.world.open_portal(e);
    let human = s.world.defs.start.as_ref().unwrap().creature_r;
    let end = tunnel[11];
    let miner = s.world.spawn_pawn(human, Faction::Player, end, None);
    s.push(rim_sim::Command::Draft { pawn: miner, on: true });
    for _ in 0..rim_sim::world::WATER_EVERY {
        s.step();
    }
    // The lake breaks in over the end of the tunnel: it fills from there
    // in a few ticks, over the miner's head.
    set(&mut s, IVec::new(end.x, end.y), "deep_water");
    let f = s.world.defs.fluids[0].clone();
    // Only the water: no wolf or raider gets to the miner first.
    let alone = |s: &mut Sim| {
        for e in s.world.pawns.clone() {
            if s.world.ecs.get::<&Pawn>(e).is_ok_and(|p| p.faction != Faction::Player) {
                let _ = s.world.ecs.despawn(e);
            }
        }
        let ecs = &s.world.ecs;
        s.world.pawns.retain(|&e| ecs.contains(e));
    };
    let mut flooded = false;
    for _ in 0..200 {
        alone(&mut s);
        s.step();
        if s.world.water_depth(end) >= f.no_air {
            flooded = true;
            break;
        }
    }
    assert!(flooded, "the tunnel floods to the roof");
    for _ in 0..3_000 {
        alone(&mut s);
        s.step();
    }
    let p = (*s.world.ecs.get::<&Pawn>(miner).expect("the miner is still here")).clone();
    assert!(!p.dead, "the miner got out alive (hp {})", p.hp);
    assert!(s.world.water_depth(p.pos) < f.wade, "and is on dry ground (at {:?})", p.pos);
}
