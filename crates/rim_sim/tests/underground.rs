//! Underground (DESIGN.md §6d): rock is a roof, the ground keeps the year's
//! mean, and nothing of the sky reaches down.

mod common;

use rim_sim::{IVec, Sim, TICKS_PER_DAY};

fn field(s: &Sim, id: &str) -> usize {
    s.world.defs.lookup("field", id).unwrap_or_else(|| panic!("field {id}")) as usize
}

fn dig(s: &mut Sim, p: IVec) {
    if let Some(leaves) = s.world.solid_at(p).and_then(|r| r.leaves_r) {
        let cost = s.world.defs.terrain[leaves as usize].path_cost;
        s.world.map.set_terrain(p, leaves, cost);
    }
}

#[test]
fn the_ground_at_minus_one_keeps_within_three_degrees_of_the_years_mean() {
    // The weather plugin's year: -6°C in midwinter to 19°C in midsummer.
    let mut s = Sim::build(&common::mods(), 3, &|m| m == "core" || m == "weather", 64).unwrap();
    let t = field(&s, "core:temperature");
    let defs = s.world.defs.clone();
    let days = defs.calendar.year_days as u64;
    let (mut surface, mut below) = (Vec::new(), Vec::new());
    for day in 0..days {
        for hour in (0..24).step_by(3) {
            s.world.tick = day * TICKS_PER_DAY + hour * TICKS_PER_DAY / 24;
            let clock = s.world.clock();
            s.world.fields.update_ambient(&defs, clock);
            surface.push(s.world.fields.outdoor(t, 0) as f64 / 100.0);
            below.push(s.world.fields.outdoor(t, -1) as f64 / 100.0);
        }
    }
    let mean = surface.iter().sum::<f64>() / surface.len() as f64;
    let (lo, hi) = below.iter().fold((f64::MAX, f64::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    let (slo, shi) = surface.iter().fold((f64::MAX, f64::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    assert!(shi - slo > 20.0, "the surface has a year to it ({slo:.1} to {shi:.1})");
    assert!(lo >= mean - 3.0 && hi <= mean + 3.0, "-1 keeps {lo:.1} to {hi:.1}, the year's mean being {mean:.1}");
}

#[test]
fn a_cellar_comes_to_the_grounds_temperature_and_is_dark() {
    let mut s = Sim::build(&common::mods(), 3, &|m| m == "core", 64).unwrap();
    let c = s.world.colony_center().unwrap();
    let mid = IVec::at(c.x, c.y, -1);
    for y in -2..=2 {
        for x in -2..=2 {
            dig(&mut s, mid.offset(x, y));
        }
    }
    for _ in 0..TICKS_PER_DAY {
        s.step();
    }
    let (t, light) = (field(&s, "core:temperature"), field(&s, "core:light"));
    let w = &s.world;
    let ground = w.fields.outdoor(t, -1) as f64 / 100.0;
    let cellar = w.fields.value(&w.defs, &w.map, t, mid);
    assert!((cellar - ground).abs() < 1.0, "a day on, the cellar is {cellar:.1}°C, the ground {ground:.1}°C");
    assert_eq!(w.fields.value(&w.defs, &w.map, light, mid), 0.0, "and no daylight reaches it");
}

#[test]
fn a_large_hall_at_minus_two_counts_as_sheltered() {
    let mut s = Sim::build(&common::mods(), 3, &|m| m == "core", 64).unwrap();
    let c = s.world.colony_center().unwrap();
    // Bigger than any roof span: rock over it is the roof.
    let o = IVec::at(c.x - 10, c.y - 10, -2);
    for y in 0..20 {
        for x in 0..20 {
            dig(&mut s, o.offset(x, y));
        }
    }
    s.world.map.ensure_rooms();
    let mid = o.offset(10, 10);
    let room = s.world.map.room_at(mid).expect("a room");
    assert!(room.enclosed(), "enclosed: {room:?}");
    assert!(s.world.map.indoors(mid), "indoors in the middle of it");
    let wind = s.world.defs.fields.iter().position(|f| f.kind == rim_sim::defs::FieldKind::Shelter);
    if let Some(wind) = wind {
        assert_eq!(s.world.fields.value(&s.world.defs, &s.world.map, wind, mid), 0.0, "and out of the wind");
    }
}

#[test]
fn in_a_hard_freeze_a_colonist_goes_down_to_the_cellar() {
    let mut s = Sim::build(&common::mods(), 3, &|m| m == "core", 64).unwrap();
    let founder = s.world.colonists().next().unwrap();
    let c = s.world.pawn_pos(founder).unwrap();
    // A cellar under open ground nearby, and stairs down to it.
    let top = (2..20)
        .flat_map(|r| [c.offset(r, 0), c.offset(-r, 0), c.offset(0, r), c.offset(0, -r)])
        .find(|&p| s.world.can_dig(p))
        .expect("somewhere to dig");
    let mid = IVec::at(top.x, top.y, -1);
    for y in -2..=2 {
        for x in -2..=2 {
            dig(&mut s, mid.offset(x, y));
        }
    }
    let stairs = s.world.defs.thing_id("stairs").unwrap();
    let e = s.world.spawn_fixture_of(stairs, top, false, None).expect("stairs");
    s.world.open_portal(e);
    // The surface freezes; the ground below keeps its 10°C.
    let t = field(&s, "core:temperature");
    s.world.fields.set_ambient(t, Some(-25.0));
    let went = (0..TICKS_PER_DAY).any(|_| {
        s.step();
        s.world.pawn_pos(founder).is_some_and(|p| p.z == -1)
    });
    assert!(went, "the founder went down out of the cold");
}
