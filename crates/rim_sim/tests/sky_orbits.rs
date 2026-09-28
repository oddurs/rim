//! Sky bodies move in the sim (DESIGN.md §6e): each body's altitude,
//! azimuth, twilight and phase follow from the tick, its orbit and the
//! calendar's latitude, with maths every platform works out alike.

mod common;

use rim_sim::defs::{CalendarDef, SkyBodyDef};
use rim_sim::sky::{self, BodyState};
use rim_sim::{Sim, TICKS_PER_DAY};

const HOUR: u64 = TICKS_PER_DAY / 24;

/// A body from its TOML, resolved alone. Its light is its own `of`, and
/// the renderer's path fields are only there because the schema wants them.
fn body(orbit: &str) -> SkyBodyDef {
    let src = format!("id = \"b\"\nof = [1.0]\nrise = 6.0\nset = 18.0\npeak = 45.0\narc = [0.0, 180.0]\n{orbit}");
    let mut b: SkyBodyDef = toml::from_str(&src).unwrap();
    b.resolve(&[], &|_: &str| None, &mut Vec::new()).unwrap();
    b
}

/// A 60-day year with its equinoxes at noon on days 0 and 30, and its
/// longest day a quarter in. The year counts from tick 0, at 06:00: in so
/// short a year the sun's declination moves 2.4° a day around an equinox,
/// so one six hours off noon would lengthen that day and shorten the other.
fn calendar() -> CalendarDef {
    CalendarDef { latitude: 45.0, midsummer: 0.25 + 0.25 / 60.0, ..CalendarDef::default() }
}

/// The tick at `hour` on `day` of the second year, so the day before is in
/// the game (tick 0 is 06:00 on day 0).
fn at(day: u64, hour: u64) -> u64 {
    (60 + day) * TICKS_PER_DAY + hour * HOUR - 6 * HOUR
}

/// Hours in the day around `day`'s noon with the body above the horizon.
fn day_length(b: &SkyBodyDef, cal: &CalendarDef, day: u64) -> f64 {
    let step = 20;
    let start = at(day, 12) - TICKS_PER_DAY / 2;
    let up = (0..TICKS_PER_DAY / step).filter(|i| sky::state(b, cal, start + i * step).altitude > 0.0).count();
    up as f64 * step as f64 / HOUR as f64
}

#[test]
fn the_suns_day_follows_the_seasons() {
    let (sun, cal) = (body("tilt = 23.0"), calendar());
    let (spring, summer, autumn, winter) =
        (day_length(&sun, &cal, 0), day_length(&sun, &cal, 15), day_length(&sun, &cal, 30), day_length(&sun, &cal, 45));
    println!("day lengths: spring {spring:.2}h summer {summer:.2}h autumn {autumn:.2}h winter {winter:.2}h");
    // At 45°N with a 23° tilt: 15.3 hours at midsummer, 8.7 at midwinter.
    assert!((summer - 15.3).abs() < 0.2, "midsummer {summer}");
    assert!((winter - 8.7).abs() < 0.2, "midwinter {winter}");
    assert!((spring - 12.0).abs() < 0.2 && (autumn - 12.0).abs() < 0.2, "equinoxes {spring} {autumn}");
    assert!((spring - autumn).abs() < 0.05, "the equinoxes are alike: {spring} {autumn}");

    // Highest at noon, due south, at 90° − latitude + declination.
    for (day, want) in [(0, 45.0), (15, 68.0), (30, 45.0), (45, 22.0)] {
        let noon = sky::state(&sun, &cal, at(day, 12));
        assert!((noon.altitude - want).abs() < 0.1, "day {day}: noon at {} not {want}", noon.altitude);
        assert!((noon.azimuth - 90.0).abs() < 0.1, "day {day}: noon due south, not {}", noon.azimuth);
        assert_eq!((noon.up, noon.phase), (1.0, 1.0), "day {day}");
    }
    // Rising in the east and setting in the west, below at midnight.
    let equinox = |h| sky::state(&sun, &cal, at(0, h));
    assert!(equinox(6).azimuth < 1.0 || equinox(6).azimuth > 359.0, "{:?}", equinox(6));
    assert!((equinox(18).azimuth - 180.0).abs() < 1.0, "{:?}", equinox(18));
    assert_eq!(equinox(0).up, 0.0);
}

#[test]
fn twilight_brings_a_body_up_gradually() {
    let (sun, cal) = (body(""), calendar());
    let ups: Vec<f64> = (0..=40).map(|m| sky::state(&sun, &cal, at(0, 5) + m * HOUR / 20).up).collect();
    assert_eq!(ups[0], 0.0, "down at 05:00");
    assert_eq!(*ups.last().unwrap(), 1.0, "up at 07:00");
    assert!(ups.windows(2).all(|w| w[1] >= w[0]), "it only rises: {ups:?}");
    assert!(ups.iter().filter(|&&u| u > 0.0 && u < 1.0).count() >= 10, "over a while, not at once");
}

#[test]
fn a_moon_wanes_and_rises_later_each_day() {
    let cal = calendar();
    let moon = body("day_period = 1.035\ntransit = 0.0\nphase_days = 15.0\nphase_offset = 0.0");
    // New at tick 0, full half a cycle on, new again a cycle on.
    let phase = |days: f64| sky::state(&moon, &cal, (days * TICKS_PER_DAY as f64) as u64).phase;
    assert!(phase(0.0) < 1e-9 && phase(15.0) < 1e-9, "new: {} {}", phase(0.0), phase(15.0));
    assert!((phase(7.5) - 1.0).abs() < 1e-9, "full: {}", phase(7.5));
    assert!((phase(3.75) - 0.5).abs() < 1e-9, "a quarter: {}", phase(3.75));

    // Each rising comes 0.035 of a day (50 minutes) after the last.
    let rises: Vec<u64> = (1..5u64)
        .map(|day| {
            let from = at(day, 0);
            let above = |t: u64| sky::state(&moon, &cal, t).altitude > 0.0;
            (from..from + TICKS_PER_DAY).find(|&t| !above(t) && above(t + 1)).expect("it rises") - from
        })
        .collect();
    for w in rises.windows(2) {
        let later = w[1] as f64 - w[0] as f64;
        assert!((later - 0.035 * TICKS_PER_DAY as f64).abs() < 0.01 * HOUR as f64, "{rises:?}");
    }
    // A body without phases is always lit.
    assert_eq!(sky::state(&body(""), &cal, 12345).phase, 1.0);
}

#[test]
fn the_maths_matches_std() {
    let mut worst = [0.0f64; 3];
    for i in -4000..=4000 {
        let x = i as f64 * 0.001_570_796;
        worst[0] = worst[0].max((sky::sin(x) - x.sin()).abs());
        worst[1] = worst[1].max((sky::cos(x) - x.cos()).abs());
    }
    for i in -100..=100 {
        for j in -100..=100 {
            let (y, x) = (i as f64 * 0.37, j as f64 * 0.29);
            worst[2] = worst[2].max((sky::atan2(y, x) - y.atan2(x)).abs());
        }
    }
    for x in [0.0, 1e-9, 0.4142, 0.4143, 1.0, 1.0001, 7.0, 1e9, f64::MAX] {
        worst[2] = worst[2].max((sky::atan(x) - x.atan()).abs()).max((sky::atan(-x) - (-x).atan()).abs());
    }
    println!("worst error: sin {:e} cos {:e} atan2 {:e}", worst[0], worst[1], worst[2]);
    assert!(worst.iter().all(|&w| w < 1e-12), "{worst:?}");
}

/// A year of states for a sun and a moon, every outdoor update, hashed bit
/// for bit. The hash is pinned: CI runs this on every platform, so a machine
/// whose arithmetic differed would fail here as well as in agree.
#[test]
fn a_year_of_body_states_is_the_same_everywhere() {
    let cal = CalendarDef { latitude: 52.0, midsummer: 0.375, start_day: 8, ..CalendarDef::default() };
    let bodies = [
        body("tilt = 23.0"),
        body("day_period = 1.07\ntransit = 24.0\ntilt = -20.0\nphase_days = 15.0\nphase_offset = 6.75"),
    ];
    let hash = || {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for tick in (0..60 * TICKS_PER_DAY).step_by(rim_sim::field::AMBIENT_INTERVAL as usize) {
            for b in &bodies {
                let BodyState { altitude, azimuth, up, phase } = sky::state(b, &cal, tick);
                for v in [altitude, azimuth, up, phase] {
                    h = (h ^ v.to_bits()).wrapping_mul(0x100_0000_01b3);
                }
            }
        }
        h
    };
    let first = hash();
    assert_eq!(first, hash(), "run to run");
    assert_eq!(first, 0xc236_f513_958b_5b4e, "pinned: {first:#x}");
}

const READS_THE_SUN: &str = r##"
[[field]]
id = "sunup"
label = "sun up"
range = [0.0, 1.0]
color_low = "#000000"
color_high = "#ffffff"
ambient = { up = { of = [{ input = "body", body = "core:sun", of = "up" }] } }

[[field]]
id = "sunlow"
label = "sun's altitude"
range = [-90.0, 90.0]
color_low = "#000000"
color_high = "#ffffff"
ambient = { alt = { of = [{ input = "body", body = "core:sun", of = "altitude" }] } }
"##;

#[test]
fn a_field_reads_where_a_body_is() {
    let dir = common::test_mods("sky-orbits", &["core"], &[("sunny", &[("defs/fields.toml", READS_THE_SUN)])]);
    let mut sim = Sim::build(&dir, 1, &|_| true, 32).unwrap();
    let defs = sim.world.defs.clone();
    let (up, alt) =
        (defs.lookup("field", "sunny:sunup").unwrap() as usize, defs.lookup("field", "sunny:sunlow").unwrap() as usize);
    let sun = defs.sky_bodies.iter().position(|b| b.id == "core:sun").unwrap();
    for _ in 0..24 {
        (0..HOUR).for_each(|_| sim.step());
        let s = sim.world.sky_body_states()[sun];
        let read = |f: usize| sim.world.fields.atmos[f].value as f64 / rim_sim::terms::Q as f64;
        assert!((read(up) - s.up).abs() < 1e-4, "up {} vs {}", read(up), s.up);
        assert!((read(alt) - s.altitude).abs() < 1e-4, "altitude {} vs {}", read(alt), s.altitude);
    }
    // The states are the tick's own: worked out afresh, they agree.
    let tick = sim.world.tick - sim.world.tick % rim_sim::field::AMBIENT_INTERVAL;
    assert_eq!(sim.world.sky_body_states(), sky::states(&defs, tick).as_slice());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_body_input_names_a_body_and_what_of_it() {
    for (bad, says) in [
        (
            READS_THE_SUN.replace("\"core:sun\", of = \"up\"", "\"core:sol\", of = \"up\""),
            "unknown sky body 'core:sol'",
        ),
        (READS_THE_SUN.replace("of = \"up\"", "of = \"colour\""), "needs `of`"),
        (
            READS_THE_SUN
                .replace("input = \"body\", body = \"core:sun\", of = \"up\"", "input = \"hour\", body = \"core:sun\""),
            "are for `input = \"body\"`",
        ),
    ] {
        let dir = common::test_mods("sky-orbits-bad", &["core"], &[("sunny", &[("defs/fields.toml", bad.as_str())])]);
        let err = Sim::build(&dir, 1, &|_| true, 32).err().expect("it doesn't load");
        assert!(err.contains(says), "{says}: {err}");
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn an_orbit_out_of_range_is_an_error() {
    for (orbit, says) in [
        ("day_period = 0.0", "day_period of an hour"),
        ("tilt = 100.0", "tilt of -90 to 90"),
        ("phase_days = 0.01", "phase_days needs an hour"),
    ] {
        let src = format!("id = \"b\"\nof = [1.0]\nrise = 6.0\nset = 18.0\npeak = 45.0\narc = [0.0, 180.0]\n{orbit}");
        let mut b: SkyBodyDef = toml::from_str(&src).unwrap();
        let err = b.resolve(&[], &|_: &str| None, &mut Vec::new()).unwrap_err();
        assert!(err.contains(says), "{says}: {err}");
    }
}

#[test]
fn a_script_reads_where_a_body_is() {
    let script = r#"rim.every(1, function()
  rim.set_data("probe:moon", rim.sky_body("core:moon"))
end)
"#;
    let dir = common::test_mods("sky-orbits-script", &["core"], &[("probe", &[("scripts/probe.luau", script)])]);
    let mut sim = Sim::new(&dir, 1).unwrap();
    (0..2).for_each(|_| sim.step());
    let moon = sim.world.defs.sky_bodies.iter().position(|b| b.id == "core:moon").unwrap();
    let want = sim.world.sky_body_states()[moon];
    let Some(got) = sim.world.data.get("probe:moon") else { panic!("{:?}", sim.world.messages) };
    let read = |k: &str| got.get(k).and_then(|d| d.num()).unwrap();
    assert_eq!(
        [read("altitude"), read("azimuth"), read("up"), read("phase")],
        [want.altitude, want.azimuth, want.up, want.phase]
    );
    let _ = std::fs::remove_dir_all(dir);
}
