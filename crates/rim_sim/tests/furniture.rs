//! Core's furniture: a table, a chair that is a seat beside it, and a
//! stove that warms a room to its cap. All content on 0212-0218.

use rim_sim::defs::DefId;
use rim_sim::hecs::Entity;
use rim_sim::world::{Blueprint, Job, MadeOf, Pawn, NEED_MAX};
use rim_sim::{Command, IVec, Sim, TICKS_PER_DAY};
use std::path::{Path, PathBuf};

fn mods() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

fn alone() -> (Sim, Entity) {
    let mut s = Sim::new(&mods(), 21).expect("mods load");
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

fn set_need(s: &mut Sim, e: Entity, id: &str, v: i32) {
    let n = s.world.defs.lookup("need", id).unwrap();
    s.world.ecs.get::<&mut Pawn>(e).unwrap().needs.iter_mut().for_each(|x| {
        if x.0 == n {
            x.1 = v;
        }
    });
}

fn open_cells(s: &Sim, n: usize) -> Vec<IVec> {
    let c = s.world.colony_center().expect("a colony");
    (1..30)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))))
        .filter(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none())
        .take(n)
        .collect()
}

fn put(s: &mut Sim, id: &str, p: IVec) -> Entity {
    let def = thing(s, id);
    s.world.spawn_fixture(def, p, false).unwrap_or_else(|| panic!("could not place {id}"))
}

fn job(s: &Sim, e: Entity) -> Job {
    s.world.ecs.get::<&Pawn>(e).unwrap().job.clone()
}

#[test]
fn all_three_are_built_of_anything_structural() {
    let (mut s, _) = alone();
    let stone = thing(&s, "stone");
    let cells = open_cells(&s, 3);
    for (id, cell) in ["table", "chair", "stove"].iter().zip(&cells) {
        let def = thing(&s, id);
        let b = s.world.defs.thing(def).build.as_ref().unwrap_or_else(|| panic!("{id} is buildable"));
        assert!(b.stuff.is_some(), "{id} takes a material, so the marble test covers it");
        s.push(Command::Build { thing: def, stuff: Some(stone), a: *cell, b: *cell });
        s.step();
        let e = s.world.map.fixture_at(*cell).unwrap_or_else(|| panic!("a stone {id} blueprint"));
        assert!(s.world.ecs.get::<&Blueprint>(e).is_ok());
        assert_eq!(s.world.ecs.get::<&MadeOf>(e).map(|m| m.0), Ok(stone), "{id} of stone");
    }
}

#[test]
fn furniture_is_walked_around_not_through_walls() {
    let (s, _) = alone();
    for id in ["table", "chair", "stove"] {
        let td = s.world.defs.thing(thing(&s, id));
        assert!(!td.blocks, "{id} is passable, so it neither bounds a room nor skews its boundary");
        assert!(td.path_cost > 0, "{id} still costs something to cross");
    }
    assert_eq!(s.world.defs.thing(thing(&s, "chair")).spots.len(), 1, "a chair is one seat");
    assert!(s.world.defs.thing(thing(&s, "table")).tags.iter().any(|t| t == "table"));
}

#[test]
fn a_pawn_eats_at_the_table() {
    let (mut s, e) = alone();
    let cells = open_cells(&s, 3);
    let table_at = cells[0];
    let chair_at = [table_at.offset(1, 0), table_at.offset(-1, 0), table_at.offset(0, 1), table_at.offset(0, -1)]
        .into_iter()
        .find(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none())
        .expect("room for a chair");
    put(&mut s, "table", table_at);
    let chair = put(&mut s, "chair", chair_at);
    let berries = thing(&s, "berries");
    s.world.place_item(berries, cells[2], 10);
    set_need(&mut s, e, "food", NEED_MAX / 10);
    let mut sat = false;
    for _ in 0..4000 {
        s.step();
        if let Job::Eat { seat: Some((c, _)), stage: 1, t, .. } = job(&s, e) {
            if c == chair && t > 0 {
                assert_eq!(s.world.pawn_pos(e).unwrap(), chair_at, "sits on the chair to eat");
                sat = true;
                break;
            }
        }
    }
    assert!(sat, "a hungry pawn with a chair at a table eats there");
}

#[test]
fn a_chair_with_no_table_is_a_stool() {
    let (mut s, e) = alone();
    let cells = open_cells(&s, 2);
    put(&mut s, "chair", cells[0]);
    let berries = thing(&s, "berries");
    s.world.place_item(berries, cells[1], 10);
    set_need(&mut s, e, "food", NEED_MAX / 10);
    for _ in 0..4000 {
        s.step();
        if let Job::Eat { seat, t, .. } = job(&s, e) {
            assert!(seat.is_none(), "no table, no seat");
            if t > 0 {
                return;
            }
        }
    }
    panic!("never ate");
}

/// Wall the founder into a 5x5 hut with a stove in it.
fn hut_with_stove(s: &mut Sim, e: Entity) -> IVec {
    let c = s.world.colony_center().expect("a colony");
    let o = (2..40)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))))
        .find(|&o| (0..5).all(|y| (0..5).all(|x| s.world.map.passable(o.offset(x, y)))))
        .expect("open ground");
    let (wall, wood) = (thing(s, "wall"), thing(s, "wood"));
    for y in 0..5 {
        for x in 0..5 {
            let p = o.offset(x, y);
            for f in [s.world.map.fixture_at(p), s.world.map.item_at(p)].into_iter().flatten() {
                s.world.despawn_thing(f);
            }
            if x == 0 || y == 0 || x == 4 || y == 4 {
                s.world.spawn_fixture_of(wall, p, false, Some(wood)).expect("wall");
            }
        }
    }
    let inside = o.offset(2, 2);
    put(s, "stove", o.offset(1, 1));
    if let Ok(mut p) = s.world.ecs.get::<&mut Pawn>(e) {
        p.pos = inside;
        p.next = None;
        p.path.clear();
        p.path_goal = None;
        p.drafted = true;
    }
    inside
}

#[test]
fn a_stove_heats_the_room_to_its_cap_and_no_further() {
    let (mut s, e) = alone();
    let inside = hut_with_stove(&mut s, e);
    let t = s.world.defs.lookup("field", "temperature").unwrap() as usize;
    let cap =
        s.world.defs.thing(thing(&s, "stove")).emit.iter().find(|m| m.field == "temperature").unwrap().cap.unwrap();
    // Six cold hours: plenty to reach the cap, and to overshoot it if nothing stopped it.
    for _ in 0..TICKS_PER_DAY / 4 {
        s.world.fields.set_ambient(t, Some(0.0));
        s.step();
    }
    let v = s.world.fields.value(&s.world.defs, &s.world.map, t, inside);
    assert!(v >= cap - 1.5, "the stove got the room up to its cap: {v}° vs {cap}°");
    assert!(v <= cap + 0.5, "and no further: {v}° vs {cap}°");
}
