//! Field layers: emitters stamped by walking distance and blocked by walls,
//! incremental updates, room values, and the warmth need built on top.

use rim_sim::field::FIXED;
use rim_sim::hecs::Entity;
use rim_sim::world::{Faction, Job, Pawn, NEED_MAX};
use rim_sim::{IVec, Sim, TICKS_PER_DAY};
use std::path::Path;

fn sim() -> Sim {
    Sim::new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods"), 21).expect("mods load")
}

fn field(s: &Sim, id: &str) -> usize {
    s.world.defs.lookup("field", id).unwrap() as usize
}

/// Emitter contribution only, in field units.
fn stamped(s: &Sim, f: usize, p: IVec) -> f64 {
    s.world.fields.layers[f].stamped[s.world.map.idx(p)] as f64 / FIXED
}

fn value(s: &Sim, f: usize, p: IVec) -> f64 {
    s.world.fields.value(&s.world.defs, &s.world.map, f, p)
}

fn put(s: &mut Sim, id: &str, p: IVec) -> Entity {
    if let Some(f) = s.world.map.fixture_at(p) {
        s.world.despawn_thing(f);
    }
    let def = s.world.defs.thing_id(id).unwrap();
    s.world.spawn_fixture(def, p, false).unwrap_or_else(|| panic!("could not place {id}"))
}

/// Flat, empty grass so distances are clean.
fn clear(s: &mut Sim, o: IVec, size: i32) {
    let grass = s.world.defs.lookup("terrain", "grass").unwrap();
    for y in 0..size {
        for x in 0..size {
            let p = o.offset(x, y);
            if let Some(f) = s.world.map.fixture_at(p) {
                s.world.despawn_thing(f);
            }
            s.world.map.set_terrain(p, grass, 100);
        }
    }
}

fn ring(s: &mut Sim, o: IVec, size: i32) {
    for y in 0..size {
        for x in 0..size {
            if x == 0 || y == 0 || x == size - 1 || y == size - 1 {
                put(s, "wall", o.offset(x, y));
            }
        }
    }
}

/// Step with the outdoor temperature pinned (the climate script would move it).
fn run_at(s: &mut Sim, temp: f64, ticks: u64) {
    let t = field(s, "temperature");
    for _ in 0..ticks {
        s.world.fields.set_ambient(t, Some(temp));
        s.step();
    }
}

fn site(s: &Sim) -> IVec {
    s.world.colony_center().unwrap().offset(12, 12)
}

#[test]
fn emitters_fade_with_walking_distance() {
    let mut s = sim();
    let o = site(&s);
    clear(&mut s, o, 17);
    let c = o.offset(8, 8);
    put(&mut s, "campfire", c);
    let t = field(&s, "temperature");
    // Campfire: 12° at the source, radius 5, fading linearly over 6 steps.
    assert_eq!(stamped(&s, t, c), 12.0);
    assert_eq!(stamped(&s, t, c.offset(3, 0)), 6.0);
    assert_eq!(stamped(&s, t, c.offset(3, 3)), 6.0, "diagonal steps count as one");
    assert_eq!(stamped(&s, t, c.offset(6, 0)), 0.0, "beyond the radius");
}

#[test]
fn walls_block_emitters() {
    let mut s = sim();
    let o = site(&s);
    clear(&mut s, o, 17);
    let c = o.offset(8, 8);
    put(&mut s, "campfire", c);
    // A wall across the east side: warmth must walk around it.
    for dy in -4..=4 {
        put(&mut s, "wall", c.offset(2, dy));
    }
    s.step();
    let t = field(&s, "temperature");
    assert_eq!(stamped(&s, t, c.offset(3, 0)), 0.0, "straight through the wall is out of reach");
    assert!(stamped(&s, t, c.offset(-3, 0)) > 0.0, "the open side still warms");

    // Fully enclosed: nothing gets out.
    let mut s = sim();
    clear(&mut s, o, 17);
    ring(&mut s, o.offset(6, 6), 5);
    put(&mut s, "campfire", o.offset(8, 8));
    s.step();
    let f = field(&s, "light");
    for p in [o.offset(5, 8), o.offset(11, 8), o.offset(8, 5), o.offset(8, 11)] {
        assert_eq!(stamped(&s, f, p), 0.0, "light leaked through a wall to {p:?}");
    }
}

#[test]
fn emitter_changes_are_incremental() {
    let mut s = sim();
    let o = site(&s);
    clear(&mut s, o, 30);
    let t = field(&s, "temperature");
    let before: i64 = s.world.fields.layers[t].stamped.iter().map(|&v| v as i64).sum();
    let fire = put(&mut s, "campfire", o.offset(8, 8));
    let touched = s.world.fields.layers[t].stamped.iter().filter(|&&v| v != 0).count();
    assert!(touched <= 11 * 11, "a radius-5 emitter touches at most 11x11 cells, touched {touched}");
    s.step();

    // A wall far away re-stamps nothing; one within reach re-stamps just this fire.
    let base = s.world.fields.restamped;
    put(&mut s, "wall", o.offset(25, 25));
    s.step();
    assert_eq!(s.world.fields.restamped, base, "a distant wall should not re-stamp the fire");
    put(&mut s, "wall", o.offset(10, 8));
    s.step();
    let redone = s.world.fields.restamped - base;
    // The fire's two emitters (heat r5: <=121 cells, light r7: <=225), once each.
    assert!(redone > 0 && redone <= 121 + 225, "re-stamped {redone} cells for one nearby wall");

    s.world.despawn_thing(fire);
    let after: i64 = s.world.fields.layers[t].stamped.iter().map(|&v| v as i64).sum();
    assert_eq!(after, before, "removing the emitter undoes exactly what it added");
}

#[test]
fn rooms_leak_toward_outdoors() {
    let mut s = sim();
    let o = site(&s);
    clear(&mut s, o, 9);
    ring(&mut s, o.offset(2, 2), 5);
    let inside = o.offset(4, 4);
    let t = field(&s, "temperature");
    // The room forms at 20°, then the night drops to 0°.
    run_at(&mut s, 20.0, 1);
    assert_eq!(value(&s, t, inside), 20.0);
    let hour = TICKS_PER_DAY / 24;
    run_at(&mut s, 0.0, hour);
    let v = value(&s, t, inside);
    // Core insulation: 6% of the gap per hour.
    assert!((18.5..19.2).contains(&v), "after an hour at 0° outside the room is {v}");
    assert_eq!(value(&s, t, o.offset(0, 0)), 0.0, "outside follows the ambient");
    run_at(&mut s, 0.0, hour * 12);
    let v = value(&s, t, inside);
    assert!((8.0..11.0).contains(&v), "after a 13-hour night at 0° an unheated room is {v}°");
}

#[test]
fn a_fire_heats_its_room() {
    let mut s = sim();
    let o = site(&s);
    clear(&mut s, o, 9);
    ring(&mut s, o.offset(2, 2), 5);
    put(&mut s, "campfire", o.offset(3, 3));
    let t = field(&s, "temperature");
    // Inside, the value is the room's own: the fire's local stamp isn't added again.
    let room = |s: &Sim| value(s, t, o.offset(5, 5));
    // Lit on a 0° night, a 3x3 hut is comfortable within about an hour...
    run_at(&mut s, 0.0, TICKS_PER_DAY / 24 * 3 / 2);
    assert!(room(&s) >= 10.0, "an hour and a half in, the hut is only {}°", room(&s));
    // ...and holds at the campfire's 24° cap, not beyond.
    run_at(&mut s, 0.0, TICKS_PER_DAY / 2);
    assert!((22.5..25.0).contains(&room(&s)), "heated room settled at {}°, outdoors 0°", room(&s));
}

#[test]
fn room_values_survive_rebuilds_elsewhere() {
    let mut s = sim();
    let o = site(&s);
    clear(&mut s, o, 20);
    ring(&mut s, o.offset(2, 2), 5);
    let t = field(&s, "temperature");
    run_at(&mut s, 20.0, 1);
    run_at(&mut s, 0.0, TICKS_PER_DAY / 24 * 3);
    let before = value(&s, t, o.offset(4, 4));
    let rebuilds = s.world.map.room_rebuilds;
    put(&mut s, "wall", o.offset(15, 15));
    run_at(&mut s, 0.0, 1);
    assert!(s.world.map.room_rebuilds > rebuilds, "the far wall rebuilt rooms");
    let after = value(&s, t, o.offset(4, 4));
    assert!((before - after).abs() < 0.2, "hut went from {before}° to {after}° when a far wall went up");
}

fn warmth(s: &Sim, e: Entity) -> i32 {
    let n = s.world.defs.lookup("need", "warmth").unwrap();
    s.world.ecs.get::<&Pawn>(e).unwrap().need(n).unwrap()
}

fn alone(s: &mut Sim) -> Entity {
    let founder = s.world.colonists().next().unwrap();
    for e in s.world.pawns.clone() {
        if e != founder {
            let _ = s.world.ecs.despawn(e);
        }
    }
    s.world.pawns.retain(|&e| e == founder);
    founder
}

#[test]
fn cold_drains_warmth_and_hurts_at_zero() {
    let mut s = sim();
    let e = alone(&mut s);
    s.push(rim_sim::Command::Draft { pawn: e, on: true }); // stand still, no seeking
    let start = warmth(&s, e);
    run_at(&mut s, 0.0, TICKS_PER_DAY / 8);
    let w = warmth(&s, e);
    assert!(w < start, "warmth should drain at 0°C: {start} -> {w}");
    let hp = s.world.ecs.get::<&Pawn>(e).unwrap().hp;
    s.world.ecs.get::<&mut Pawn>(e).unwrap().needs.iter_mut().for_each(|n| {
        if n.0 == s.world.defs.lookup("need", "warmth").unwrap() {
            n.1 = 0;
        }
    });
    run_at(&mut s, -10.0, TICKS_PER_DAY / 8);
    assert!(s.world.ecs.get::<&Pawn>(e).unwrap().hp < hp, "hypothermia should hurt");
}

#[test]
fn comfortable_ground_restores_warmth() {
    let mut s = sim();
    let e = alone(&mut s);
    s.push(rim_sim::Command::Draft { pawn: e, on: true });
    let n = s.world.defs.lookup("need", "warmth").unwrap();
    s.world.ecs.get::<&mut Pawn>(e).unwrap().needs.iter_mut().for_each(|x| {
        if x.0 == n {
            x.1 = NEED_MAX / 5;
        }
    });
    run_at(&mut s, 20.0, TICKS_PER_DAY / 24);
    assert!(warmth(&s, e) > NEED_MAX / 2, "an hour at 20° should warm a pawn up");
}

#[test]
fn cold_colonists_go_to_the_fire() {
    let mut s = sim();
    let e = alone(&mut s);
    let home = s.world.pawn_pos(e).unwrap();
    let t = field(&s, "temperature");
    // A fire some distance away on open ground.
    let spot = (6..20)
        .flat_map(|r| [home.offset(r, 0), home.offset(-r, 0), home.offset(0, r), home.offset(0, -r)])
        .find(|p| s.world.map.passable(*p) && s.world.map.fixture_at(*p).is_none())
        .expect("room for a fire");
    put(&mut s, "campfire", spot);
    let n = s.world.defs.lookup("need", "warmth").unwrap();
    s.world.ecs.get::<&mut Pawn>(e).unwrap().needs.iter_mut().for_each(|x| {
        if x.0 == n {
            x.1 = NEED_MAX / 5;
        }
    });
    // The pawn finishes warming and wanders off again, so where it stands
    // at a fixed tick proves nothing. What matters is that it sought
    // warmth, was somewhere warm while doing so, and warmed up.
    let (mut sought, mut warmest, mut peak) = (false, f64::MIN, 0);
    for _ in 0..1500 {
        run_at(&mut s, 2.0, 1);
        let p = s.world.ecs.get::<&Pawn>(e).unwrap();
        if matches!(p.job, Job::Comfort { .. }) {
            sought = true;
            warmest = warmest.max(value(&s, t, p.pos));
        }
        peak = peak.max(warmth(&s, e));
    }
    assert!(sought, "a cold colonist should go looking for warmth");
    assert!(warmest >= 11.0, "should have stood somewhere warm while warming up, best was {warmest}°");
    assert!(peak > NEED_MAX / 2, "and warmed up, peak {peak}");
}

#[test]
fn core_climate_drives_the_day() {
    // Core alone: the weather plugin would add seasons and clouds.
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let mut s = Sim::with_mods(&mods, 21, &|m| m == "core").expect("core loads");
    let t = field(&s, "temperature");
    let l = field(&s, "light");
    let mut coldest = f64::MAX;
    let mut warmest = f64::MIN;
    let mut dark = false;
    for _ in 0..TICKS_PER_DAY {
        s.step();
        let a = s.world.fields.ambient(t);
        coldest = coldest.min(a);
        warmest = warmest.max(a);
        dark |= s.world.fields.ambient(l) == 0.0;
    }
    // Core climate: mean 10°, swinging 9° either way (03:00 coldest, 15:00 warmest).
    assert!((0.5..2.0).contains(&coldest) && (18.0..19.5).contains(&warmest), "day ran {coldest}..{warmest}°");
    assert!(dark, "nights get dark");
    let _ = Faction::Player;
}
