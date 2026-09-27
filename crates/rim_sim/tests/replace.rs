//! Replace in place (DESIGN.md §6c): planning a wall over a standing one of
//! the same kind swaps them in one step when the new one is built, so the
//! room is never open to the weather.

mod common;

use rim_sim::hecs::Entity;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{MadeOf, Replaces};
use rim_sim::{Command, IVec, Sim};

fn core() -> Sim {
    Sim::with_mods(&common::mods(), 7, &|m| m == "core").expect("mods load")
}

/// A finished wood hut, 5 across, its door in the middle of the south
/// wall, on open ground near the colony. Its corner and door.
fn hut(s: &mut Sim) -> (IVec, IVec) {
    let c = s.world.colony_center().unwrap();
    let open = |s: &Sim, p: IVec| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none();
    let o = (4..40)
        .flat_map(|r| [c.offset(r, 0), c.offset(-r, 0), c.offset(0, r), c.offset(0, -r)])
        .find(|&o| (-1..6).all(|y| (-1..6).all(|x| open(s, o.offset(x, y)))))
        .expect("open ground");
    let d = &s.world.defs;
    let (wall, door, wood) = (d.thing_id("wall").unwrap(), d.thing_id("door").unwrap(), d.thing_id("wood").unwrap());
    let doorway = o.offset(2, 4);
    for y in 0..5 {
        for x in 0..5 {
            let p = o.offset(x, y);
            if x == 0 || y == 0 || x == 4 || y == 4 {
                let def = if p == doorway { door } else { wall };
                s.world.spawn_fixture_of(def, p, false, Some(wood)).expect("a piece");
            }
        }
    }
    (o, doorway)
}

fn indoors(s: &mut Sim, p: IVec) -> bool {
    s.world.map.ensure_rooms();
    s.world.map.room_at(p).is_some_and(|r| r.enclosed())
}

fn made_of(s: &Sim, p: IVec) -> Option<rim_sim::defs::DefId> {
    s.world.map.fixture_at(p).and_then(|f| s.world.ecs.get::<&MadeOf>(f).ok().map(|m| m.0))
}

fn draw_ring(s: &mut Sim, o: IVec, stuff: rim_sim::defs::DefId) {
    let wall = s.world.defs.thing_id("wall").unwrap();
    for (a, b) in [
        (o, o.offset(4, 0)),
        (o.offset(0, 4), o.offset(4, 4)),
        (o.offset(0, 1), o.offset(0, 3)),
        (o.offset(4, 1), o.offset(4, 3)),
    ] {
        s.push(Command::Build { thing: wall, stuff: Some(stuff), a, b, facing: 0 });
    }
    s.step();
}

#[test]
fn a_hut_rebuilt_in_stone_stays_indoors_through_every_swap() {
    let mut s = core();
    let (o, doorway) = hut(&mut s);
    let inside = o.offset(2, 2);
    assert!(indoors(&mut s, inside), "the hut is a room to start with");
    let d = &s.world.defs;
    let (door, stone) = (d.thing_id("door").unwrap(), d.thing_id("stone").unwrap());
    s.world.place_item(stone, o.offset(-2, 2), 100);
    draw_ring(&mut s, o, stone);
    let pending = |s: &Sim| s.world.ecs.query::<&Replaces>().iter().count();
    assert_eq!(pending(&s), 15, "every wall is planned over; the door is kept");
    let edge = |x: i32, y: i32| x == 0 || y == 0 || x == 4 || y == 4;
    let ring: Vec<IVec> = (0..5)
        .flat_map(|y| (0..5).map(move |x| (x, y)))
        .filter(|&(x, y)| edge(x, y))
        .map(|(x, y)| o.offset(x, y))
        .collect();
    for tick in 0..40_000 {
        s.step();
        assert!(indoors(&mut s, inside), "indoors at tick {tick}, {} swaps to go", pending(&s));
        if pending(&s) == 0 {
            break;
        }
    }
    assert_eq!(pending(&s), 0, "every wall swapped");
    for p in ring {
        if p == doorway {
            assert_eq!(s.world.map.fixture_at(p).and_then(|f| s.world.thing(f)).map(|t| t.def), Some(door));
        } else {
            assert_eq!(made_of(&s, p), Some(stone), "stone at {p:?}");
        }
        assert!(s.world.map.item_at(p).is_none(), "nothing refunded under the wall at {p:?}");
    }
}

#[test]
fn cancelling_a_replacement_leaves_the_old_wall() {
    let mut s = core();
    let (o, _) = hut(&mut s);
    let d = &s.world.defs;
    let (wall, wood, stone) = (d.thing_id("wall").unwrap(), d.thing_id("wood").unwrap(), d.thing_id("stone").unwrap());
    let p = o.offset(0, 2);
    let old = s.world.map.fixture_at(p).unwrap();
    s.push(Command::Build { thing: wall, stuff: Some(stone), a: p, b: p, facing: 0 });
    s.step();
    assert!(s.world.replacement_of(old).is_some(), "planned over");
    assert_eq!(s.world.map.fixture_at(p), Some(old), "the old wall still holds the cell");
    s.push(Command::Cancel { a: p, b: p });
    s.step();
    assert!(s.world.replacement_of(old).is_none(), "the plan is gone");
    assert_eq!(s.world.map.fixture_at(p), Some(old));
    assert_eq!(made_of(&s, p), Some(wood));
}

#[test]
fn one_click_puts_a_door_into_a_wall_and_a_save_keeps_it() {
    let mut s = core();
    let (o, _) = hut(&mut s);
    let d = &s.world.defs;
    let (door, wood) = (d.thing_id("door").unwrap(), d.thing_id("wood").unwrap());
    let p = o.offset(0, 2);
    let old = s.world.map.fixture_at(p).unwrap();
    s.push(Command::Build { thing: door, stuff: Some(wood), a: p, b: p, facing: 0 });
    s.step();
    let bp: Entity = s.world.replacement_of(old).expect("a door planned into the wall");
    let dir = common::mods();
    let snap = Snapshot::capture(&s);
    let back = snap.restore(&dir, &|m| m == "core").unwrap_or_else(|e| panic!("restores: {e}"));
    assert_eq!(Snapshot::capture(&back).hash(), snap.hash());
    assert_eq!(back.world.map.fixture_at(p), Some(old), "the wall, not its replacement, holds the cell");
    assert_eq!(back.world.replacement_of(old), Some(bp));
}
