//! Floors: built ground that sits under everything, changes what a step
//! costs, and is otherwise invisible to rooms, fields and raiders.

use rim_sim::defs::{DefId, Targets};
use rim_sim::hecs::Entity;
use rim_sim::order;
use rim_sim::path::Goal;
use rim_sim::world::{Blueprint, Job, MadeOf, Pawn, Thing};
use rim_sim::{Command, IVec, Sim};
use std::path::Path;

fn sim() -> (Sim, Entity) {
    let mut s = Sim::new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods"), 21).expect("mods load");
    s.step();
    let founder = s.world.colonists().next().expect("a founder");
    for e in s.world.pawns.clone() {
        if e != founder {
            let _ = s.world.ecs.despawn(e);
        }
    }
    s.world.pawns.retain(|&e| e == founder);
    (s, founder)
}

fn thing(s: &Sim, id: &str) -> DefId {
    s.world.defs.thing_id(id).unwrap_or_else(|| panic!("thing {id}"))
}

fn open_cells(s: &Sim, n: usize) -> Vec<IVec> {
    let c = s.world.colony_center().expect("a colony");
    (1..30)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))))
        .filter(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none())
        .take(n)
        .collect()
}

/// Build it the way the colony would: owned, made of `stuff`.
fn built(s: &mut Sim, id: &str, stuff: DefId, at: IVec) -> Entity {
    let def = thing(s, id);
    let e = s.world.spawn_fixture_of(def, at, false, Some(stuff)).unwrap_or_else(|| panic!("place {id}"));
    rim_sim::ai::complete_building(&mut s.world, e);
    e
}

fn stacks_near(s: &Sim, def: DefId, near: IVec) -> u32 {
    s.world
        .ecs
        .query::<&Thing>()
        .without::<&Blueprint>()
        .iter()
        .filter(|t| t.def == def && t.pos.chebyshev(near) <= 2)
        .map(|t| t.count)
        .sum()
}

#[test]
fn a_floor_and_a_bed_share_a_cell_and_so_do_a_floor_and_a_wall() {
    let (mut s, _) = sim();
    let (wood, stone) = (thing(&s, "wood"), thing(&s, "stone"));
    let cells = open_cells(&s, 2);
    let f = built(&mut s, "floor", stone, cells[0]);
    let b = built(&mut s, "bed", wood, cells[0]);
    assert_eq!(s.world.map.floor_at(cells[0]), Some(f), "the floor is in its own layer");
    assert_eq!(s.world.map.fixture_at(cells[0]), Some(b), "the bed is on top of it");
    assert_eq!(s.world.ecs.get::<&MadeOf>(f).map(|m| m.0), Ok(stone), "and the floor knows it is stone");

    let f2 = built(&mut s, "floor", wood, cells[1]);
    let w = built(&mut s, "wall", stone, cells[1]);
    assert_eq!(s.world.map.floor_at(cells[1]), Some(f2));
    assert_eq!(s.world.map.fixture_at(cells[1]), Some(w));
    assert!(!s.world.map.passable(cells[1]), "the wall still blocks, floor or no floor");
    // A second floor on the same cell is refused, like a second fixture would be.
    assert!(s.world.spawn_fixture_of(thing(&s, "floor"), cells[0], false, Some(wood)).is_none());
}

#[test]
fn walking_over_a_floor_costs_what_the_floor_says() {
    let (mut s, _) = sim();
    let stone = thing(&s, "stone");
    let at = open_cells(&s, 1)[0];
    let ground = s.world.map.cost(at);
    let floor_cost = s.world.defs.thing(thing(&s, "floor")).path_cost;
    assert!(floor_cost < ground, "core's floor is faster than the ground it is on ({floor_cost} vs {ground})");
    let f = built(&mut s, "floor", stone, at);
    assert_eq!(s.world.map.cost(at), floor_cost, "the floor's cost stands in for the terrain's");
    s.world.despawn_thing(f);
    assert_eq!(s.world.map.cost(at), ground, "lift it and the ground is back");
    // A blueprint changes nothing yet.
    s.world.spawn_fixture_of(thing(&s, "floor"), at, true, Some(stone)).expect("a floor blueprint");
    assert_eq!(s.world.map.cost(at), ground, "a planned floor is still grass");
}

#[test]
fn rooms_and_boundaries_do_not_notice_floors() {
    let (mut s, _) = sim();
    let (wood, stone) = (thing(&s, "wood"), thing(&s, "stone"));
    let c = s.world.colony_center().expect("a colony");
    let o = (2..40)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))))
        .find(|&o| (0..5).all(|y| (0..5).all(|x| s.world.map.passable(o.offset(x, y)))))
        .expect("open ground");
    for y in 0..5 {
        for x in 0..5 {
            let p = o.offset(x, y);
            for e in [s.world.map.fixture_at(p), s.world.map.item_at(p)].into_iter().flatten() {
                s.world.despawn_thing(e);
            }
            if x == 0 || y == 0 || x == 4 || y == 4 {
                built(&mut s, "wall", wood, p);
            }
        }
    }
    s.world.map.ensure_rooms();
    s.world.map.ensure_regions();
    s.world.refresh_boundaries();
    let inside = o.offset(2, 2);
    let room = s.world.map.room_at(inside).expect("a room");
    let t = s.world.defs.lookup("field", "temperature").unwrap() as usize;
    let outside = o.offset(2, 6);
    let reach_before = s.world.map.can_reach(outside, Goal::Touch(inside));
    let before = (room.cells, s.world.map.room_boundary(room.id).len(), s.world.fields.boundary(t, room.id));

    // Floor the whole interior in stone.
    for y in 1..4 {
        for x in 1..4 {
            built(&mut s, "floor", stone, o.offset(x, y));
        }
    }
    s.step();
    let room = s.world.map.room_at(inside).expect("still a room");
    assert!(room.enclosed(), "still enclosed");
    let after = (room.cells, s.world.map.room_boundary(room.id).len(), s.world.fields.boundary(t, room.id));
    assert_eq!(before, after, "cells, boundary pieces and leak are exactly as they were");
    assert_eq!(s.world.map.can_reach(outside, Goal::Touch(inside)), reach_before, "and reachability is as it was");
}

#[test]
fn deconstructing_a_floor_gives_the_material_back() {
    let (mut s, founder) = sim();
    let (stone, wood) = (thing(&s, "stone"), thing(&s, "wood"));
    let at = open_cells(&s, 1)[0];
    let f = built(&mut s, "floor", stone, at);
    let (stone0, wood0) = (stacks_near(&s, stone, at), stacks_near(&s, wood, at));
    let decon =
        s.world.defs.designations.iter().position(|d| d.targets == Targets::Built).expect("deconstruct") as DefId;
    s.push(Command::Designate { designation: decon, a: at, b: at });
    s.step();
    assert!(
        s.world.ecs.get::<&rim_sim::world::Designated>(f).is_ok(),
        "the floor is marked, though it is not a fixture"
    );
    let mut took = false;
    for _ in 0..4000 {
        s.step();
        took |=
            matches!(s.world.ecs.get::<&Pawn>(founder).unwrap().job, Job::Deconstruct { target, .. } if target == f);
        if s.world.thing(f).is_none() {
            break;
        }
    }
    assert!(took && s.world.thing(f).is_none(), "the floor came up");
    assert_eq!(s.world.map.floor_at(at), None);
    let refund = s.world.defs.thing(thing(&s, "floor")).build.as_ref().unwrap().refund;
    assert_eq!(stacks_near(&s, stone, at) - stone0, (2.0 * refund).round() as u32, "stone back, not wood");
    assert_eq!(stacks_near(&s, wood, at), wood0);
}

#[test]
fn right_click_on_a_floor_offers_to_take_it_up() {
    let (mut s, founder) = sim();
    let stone = thing(&s, "stone");
    let at = open_cells(&s, 1)[0];
    let f = built(&mut s, "floor", stone, at);
    let o = order::resolve(&s.world, founder, at, None).expect("an order on our floor");
    assert_eq!(o.label, "Deconstruct floor");
    assert!(matches!(o.job, Job::Deconstruct { target, .. } if target == f));
}

#[test]
fn cancelling_a_floor_blueprint_refunds_what_was_delivered() {
    let (mut s, _) = sim();
    let stone = thing(&s, "stone");
    let at = open_cells(&s, 1)[0];
    let bp = s.world.spawn_fixture_of(thing(&s, "floor"), at, true, Some(stone)).expect("a floor blueprint");
    s.world.ecs.get::<&mut Blueprint>(bp).unwrap().delivered[0] = 1;
    let before = stacks_near(&s, stone, at);
    s.push(Command::Cancel { a: at, b: at });
    s.step();
    assert_eq!(s.world.map.floor_at(at), None, "cancelled");
    assert_eq!(stacks_near(&s, stone, at) - before, 1, "the one stone delivered comes back");
}
