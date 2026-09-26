//! Derived fields and feels-like temperature: what the cold feels like
//! where you stand, wind and rain included.

mod common;

use rim_sim::hecs::Entity;
use rim_sim::world::Pawn;
use rim_sim::{IVec, Sim, TICKS_PER_DAY};

fn field(s: &Sim, id: &str) -> usize {
    s.world.defs.lookup("field", id).unwrap_or_else(|| panic!("field {id}")) as usize
}

fn at(s: &Sim, id: &str, p: IVec) -> f64 {
    s.world.fields.value(&s.world.defs, &s.world.map, field(s, id), p)
}

/// Pin the weather: air temperature, wind (blowing east) and rain.
fn weather(s: &mut Sim, temp: f64, wind: f64, rain: f64) {
    for (id, v) in [("temperature", temp), ("wind", wind), ("wind_dir", 0.0), ("precipitation", rain)] {
        let f = field(s, id);
        s.world.fields.set_ambient(f, Some(v));
    }
}

/// Passable ground near the colony, cleared of trees and rock, from 4
/// cells west to `n` east and 2 north and south.
fn open_ground(s: &mut Sim, n: i32) -> IVec {
    let c = s.world.colony_center().unwrap();
    let cells = move |o: IVec| (-4..n).flat_map(move |x| (-2..=2).map(move |y| o.offset(x, y)));
    let o = (3..60)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))))
        .find(|&o| cells(o).all(|p| s.world.map.passable(p) || s.world.map.fixture_at(p).is_some()))
        .expect("open ground");
    for p in cells(o) {
        if let Some(f) = s.world.map.fixture_at(p) {
            s.world.despawn_thing(f);
        }
    }
    o
}

#[test]
fn in_still_dry_air_it_feels_like_the_thermometer() {
    let mut s = Sim::new(&common::mods(), 3).unwrap();
    weather(&mut s, 4.0, 0.0, 0.0);
    s.step();
    let p = open_ground(&mut s, 2);
    assert_eq!(at(&s, "feels_like", p), at(&s, "temperature", p));
    let f = field(&s, "feels_like");
    assert_eq!(s.world.fields.ambient(f), 4.0, "the outdoor value is worked out the same way");
}

#[test]
fn the_lee_of_a_wall_is_warmer_in_a_cold_wind() {
    let mut s = Sim::new(&common::mods(), 3).unwrap();
    weather(&mut s, 0.0, 16.0, 0.0);
    s.step();
    let o = open_ground(&mut s, 8);
    let open = at(&s, "feels_like", o.offset(2, 0));
    assert!((open - -3.0).abs() < 0.05, "16 m/s at 0° in the open is -3°: {open}");
    // A short wall across the wind, which blows east.
    let (wall, wood) = (s.world.defs.thing_id("wall").unwrap(), s.world.defs.thing_id("wood"));
    for dy in -1..=1 {
        s.world.spawn_fixture_of(wall, o.offset(1, dy), false, wood);
    }
    s.step();
    let lee = at(&s, "feels_like", o.offset(2, 0));
    let upwind = at(&s, "feels_like", o.offset(0, 0));
    assert!(lee > open + 2.5, "just behind the wall it's nearly still: {lee}° vs {open}° in the open");
    assert_eq!(upwind, open, "the windward side gets all of it");
    assert!(at(&s, "feels_like", o.offset(6, 0)) < lee, "the lee fades with distance");
}

#[test]
fn cold_rain_feels_colder_and_warm_rain_does_not() {
    let mut s = Sim::new(&common::mods(), 3).unwrap();
    let p = open_ground(&mut s, 2);
    weather(&mut s, 2.0, 0.0, 6.0);
    s.step();
    assert!((at(&s, "feels_like", p) - 0.8).abs() < 0.05, "6 mm/h at 2° takes 1.2° off");
    weather(&mut s, 2.0, 0.0, 1.5);
    s.step();
    assert_eq!(at(&s, "feels_like", p), 2.0, "a drizzle doesn't soak through");
    weather(&mut s, 25.0, 0.0, 6.0);
    s.step();
    assert_eq!(at(&s, "feels_like", p), 25.0, "a summer shower is just wet");
}

#[test]
fn indoors_it_feels_like_the_room() {
    let mut s = Sim::new(&common::mods(), 3).unwrap();
    let o = open_ground(&mut s, 6);
    let (wall, wood) = (s.world.defs.thing_id("wall").unwrap(), s.world.defs.thing_id("wood"));
    for y in -2i32..=2 {
        for x in 0..5 {
            if x == 0 || x == 4 || y.abs() == 2 {
                s.world.spawn_fixture_of(wall, o.offset(x, y), false, wood);
            }
        }
    }
    weather(&mut s, -5.0, 16.0, 6.0);
    for _ in 0..200 {
        s.step();
    }
    let inside = o.offset(2, 0);
    assert!(s.world.map.room_at(inside).is_some_and(|r| r.enclosed()));
    assert_eq!(at(&s, "feels_like", inside), at(&s, "temperature", inside), "no wind or rain gets in");
    assert!(at(&s, "feels_like", o.offset(-3, 0)) < at(&s, "temperature", o.offset(-3, 0)) - 2.0);
}

fn warmth_after(storm: bool) -> i32 {
    let mut s = Sim::new(&common::mods(), 3).unwrap();
    let e: Entity = s.world.colonists().next().unwrap();
    s.push(rim_sim::Command::Draft { pawn: e, on: true }); // stand still, no seeking
    let (wind, rain) = if storm { (16.0, 6.0) } else { (0.0, 0.0) };
    for _ in 0..TICKS_PER_DAY / 12 {
        weather(&mut s, 2.0, wind, rain);
        s.step();
    }
    let n = s.world.defs.lookup("need", "warmth").unwrap();
    let p = s.world.ecs.get::<&Pawn>(e).unwrap();
    p.need(n).unwrap()
}

#[test]
fn warmth_drains_faster_in_a_cold_storm_than_on_a_still_night() {
    let (still, storm) = (warmth_after(false), warmth_after(true));
    assert!(storm < still, "the same 2° with a gale and rain: warmth {storm} vs {still} on a still night");
}

#[test]
fn a_mod_derives_a_field_from_data() {
    let dir = common::test_mods(
        "derived",
        &["core"],
        &[(
            "heat_index",
            &[(
                "defs/fields.toml",
                r##"
[[field]]
id = "doubled"
label = "doubled"
kind = "derived"
range = [0.0, 1.0]
color_low = "#000000"
color_high = "#ffffff"

[field.value.twice]
scale = 2.0
of = [{ field = "core:temperature" }]

[field.value.plus_light]
scale = 0.01
of = [{ field = "core:light" }]
"##,
            )],
        )],
    );
    let mut s = Sim::new(&dir, 3).unwrap();
    s.step();
    let c = s.world.colony_center().unwrap();
    for p in [c, c.offset(5, 3), c.offset(-9, 4)] {
        let want = 2.0 * at(&s, "core:temperature", p) + 0.01 * at(&s, "core:light", p);
        assert!((at(&s, "heat_index:doubled", p) - want).abs() < 0.02, "at {p:?}");
    }
    let _ = std::fs::remove_dir_all(dir);

    let bad = |name: &str, body: &str| {
        let dir = common::test_mods(name, &["core"], &[("bad", &[("defs/f.toml", body)])]);
        let e = Sim::new(&dir, 1).err().expect("refused");
        let _ = std::fs::remove_dir_all(dir);
        e
    };
    let head =
        "[[field]]\nid = \"x\"\nlabel = \"x\"\nrange = [0.0, 1.0]\ncolor_low = \"#000000\"\ncolor_high = \"#ffffff\"\n";
    let e = bad("derived-ambient", &format!("{head}kind = \"derived\"\nambient = 3.0\n"));
    assert!(e.contains("gives `value` terms"), "{e}");
    let e = bad("derived-kind", &format!("{head}[field.value.a]\nof = [1.0]\n"));
    assert!(e.contains("is for kind = \"derived\""), "{e}");
    let cycle = format!("{head}kind = \"derived\"\n[field.value.a]\nof = [{{ field = \"x\" }}]\n");
    assert!(bad("derived-cycle", &cycle).contains("cycle"));
    let e = bad("derived-room", &format!("{head}kind = \"derived\"\nindoor = \"room\"\n[field.value.a]\nof = [1.0]\n"));
    assert!(e.contains("no `ambient` or `indoor`"), "{e}");
    let stove = "[[patch]]\ntarget = \"thing/core:campfire\"\nappend = { emit = [{ field = \"core:feels_like\", amount = 5.0, radius = 2 }] }\n";
    let e = bad("derived-emit", stove);
    assert!(e.contains("worked out from others"), "{e}");
}
