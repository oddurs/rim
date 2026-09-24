//! Windows: a wall with a pane in it. Shut to feet, open to daylight, the
//! draughtiest piece of the room, and the first thing a raider goes for.

use rim_sim::defs::DefId;
use rim_sim::hecs::Entity;
use rim_sim::path::Goal;
use rim_sim::world::{Faction, Job, Pawn};
use rim_sim::{IVec, Sim, TICKS_PER_DAY};
use std::path::{Path, PathBuf};

fn mods() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

fn sim() -> Sim {
    Sim::new(&mods(), 21).expect("mods load")
}

fn field(s: &Sim, id: &str) -> usize {
    s.world.defs.lookup("field", id).unwrap_or_else(|| panic!("field {id}")) as usize
}

fn thing(s: &Sim, id: &str) -> DefId {
    s.world.defs.thing_id(id).unwrap_or_else(|| panic!("thing {id}"))
}

fn site(s: &Sim, taken: &[IVec]) -> IVec {
    let c = s.world.colony_center().expect("a colony");
    (2..40)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))))
        .find(|&o| {
            (0..5).all(|y| (0..5).all(|x| s.world.map.passable(o.offset(x, y))))
                && taken.iter().all(|t| (t.x - o.x).abs() >= 7 || (t.y - o.y).abs() >= 7)
        })
        .expect("open ground for a hut")
}

/// Build it the way the colony would, so it is owned. No material, so hp
/// is the def's: window 60, door 150, wall 200.
fn build(s: &mut Sim, def: DefId, p: IVec) -> Entity {
    if let Some(f) = s.world.map.fixture_at(p) {
        s.world.despawn_thing(f);
    }
    if let Some(i) = s.world.map.item_at(p) {
        s.world.despawn_thing(i);
    }
    let e = s.world.spawn_fixture(def, p, false).expect("piece placed");
    rim_sim::ai::complete_building(&mut s.world, e);
    e
}

/// A 5x5 hut at `o`. Returns (inside, door, window) entities where placed.
fn hut(s: &mut Sim, o: IVec, door: Option<IVec>, window: Option<IVec>) -> (IVec, Option<Entity>, Option<Entity>) {
    let (wall, door_def, win_def) = (thing(s, "wall"), thing(s, "door"), thing(s, "window"));
    let (mut d, mut w) = (None, None);
    for y in 0..5 {
        for x in 0..5 {
            let p = o.offset(x, y);
            if !(x == 0 || y == 0 || x == 4 || y == 4) {
                for e in [s.world.map.fixture_at(p), s.world.map.item_at(p)].into_iter().flatten() {
                    s.world.despawn_thing(e);
                }
                continue;
            }
            if Some(p) == door {
                d = Some(build(s, door_def, p));
            } else if Some(p) == window {
                w = Some(build(s, win_def, p));
            } else {
                build(s, wall, p);
            }
        }
    }
    s.world.map.ensure_rooms();
    s.world.map.ensure_regions();
    s.world.refresh_boundaries();
    (o.offset(2, 2), d, w)
}

fn room_id(s: &Sim, p: IVec) -> u32 {
    let r = s.world.map.room_at(p).expect("a room");
    assert!(r.enclosed(), "enclosed");
    r.id
}

fn value(s: &Sim, f: usize, p: IVec) -> f64 {
    s.world.fields.value(&s.world.defs, &s.world.map, f, p)
}

fn colonists_inside(s: &mut Sim, at: IVec) {
    for e in s.world.colonists().collect::<Vec<_>>() {
        if let Ok(mut p) = s.world.ecs.get::<&mut Pawn>(e) {
            p.pos = at;
            p.next = None;
            p.path.clear();
            p.path_goal = None;
        }
    }
}

#[test]
fn a_room_with_a_window_is_lit_by_day() {
    let mut s = sim();
    let a = site(&s, &[]);
    let b = site(&s, &[a]);
    let (dark, _, _) = hut(&mut s, a, None, None);
    let (lit, _, _) = hut(&mut s, b, None, Some(b.offset(2, 0)));
    let light = field(&s, "light");
    s.world.fields.set_ambient(light, Some(100.0));
    for _ in 0..25 {
        s.step();
    }
    let (d, l) = (value(&s, light, dark), value(&s, light, lit));
    assert!(d < 1.0, "no window, no daylight: {d}");
    assert!(l >= 30.0, "one window lights the room to a third: {l}");
}

#[test]
fn a_window_loses_heat_faster_than_a_wall() {
    let mut s = sim();
    let a = site(&s, &[]);
    let b = site(&s, &[a]);
    let (sealed, _, _) = hut(&mut s, a, None, None);
    let (windowed, _, _) = hut(&mut s, b, None, Some(b.offset(2, 0)));
    let t = field(&s, "temperature");
    let (l0, _) = s.world.fields.boundary(t, room_id(&s, sealed));
    let (l1, _) = s.world.fields.boundary(t, room_id(&s, windowed));
    assert!(l1 > l0, "the window leaks: {l1} vs {l0}");

    // Warm both to the same value, then let them cool in the same cold.
    // (Room values are allocated on the first field step.)
    s.step();
    for r in [sealed, windowed] {
        let id = room_id(&s, r);
        s.world.fields.layers[t].rooms[id as usize - 1] = 2400;
    }
    for _ in 0..TICKS_PER_DAY / 8 {
        s.world.fields.set_ambient(t, Some(0.0));
        s.step();
    }
    let (ts, tw) = (value(&s, t, sealed), value(&s, t, windowed));
    assert!(ts > tw + 0.5, "three hours on, the windowed room is colder: {tw}° vs {ts}°");
}

#[test]
fn nobody_walks_through_a_window() {
    let mut s = sim();
    let a = site(&s, &[]);
    let (_, _, win) = hut(&mut s, a, None, Some(a.offset(2, 0)));
    let p = s.world.thing(win.expect("window")).expect("placed").pos;
    for who in Faction::ALL {
        assert!(!s.world.map.passable_for(p, who), "{who:?} cannot walk through a window");
    }
}

#[test]
fn a_raider_goes_for_the_window_before_the_door() {
    let mut s = sim();
    let a = site(&s, &[]);
    let (inside, door, win) = hut(&mut s, a, Some(a.offset(2, 4)), Some(a.offset(2, 0)));
    colonists_inside(&mut s, inside);
    // Stand it beside the door, so nearest would pick the door.
    let outside = a.offset(2, 6);
    let human = s.world.defs.creature_id("human").expect("human");
    let raider = s.world.spawn_pawn(human, Faction::Hostile, outside, Some("Testrunner".into()));
    for _ in 0..120 {
        s.step();
    }
    let job = s.world.ecs.get::<&Pawn>(raider).expect("alive").job.clone();
    let (door, win) = (door.expect("door"), win.expect("window"));
    assert!(
        matches!(job, Job::Breach { target } if target == win),
        "the window is the weak point, not the door: {job:?}"
    );
    assert!(s.world.thing(door).is_some());
}

#[test]
fn a_sealed_hut_gets_its_wall_broken() {
    let mut s = sim();
    let a = site(&s, &[]);
    let (inside, _, _) = hut(&mut s, a, None, None);
    colonists_inside(&mut s, inside);
    let outside = a.offset(2, 7);
    let human = s.world.defs.creature_id("human").expect("human");
    let raider = s.world.spawn_pawn(human, Faction::Hostile, outside, Some("Testrunner".into()));
    for _ in 0..120 {
        s.step();
    }
    let job = s.world.ecs.get::<&Pawn>(raider).expect("alive").job.clone();
    let Job::Breach { target } = job else { panic!("with no door or window, a raider digs through the wall: {job:?}") };
    let pos = s.world.thing(target).expect("the piece").pos;
    assert!(
        s.world.map.room_boundary(room_id(&s, inside)).contains(&(s.world.map.idx(pos) as u32)),
        "and it is a piece of this hut"
    );
    assert!(s.world.map.can_reach_for(outside, Goal::Touch(pos), Faction::Hostile));
}
