//! Sky bodies (DESIGN.md §6e): data the renderer lights the world from, each
//! as bright as one labelled term of a field (the sim's light too) or terms
//! of its own (the picture's alone).

mod common;

use rim_sim::Sim;

const GREEN_MOON: &str = r##"
[[patch]]
target = "field/core:daylight"

[patch.set.ambient.green_moon]
scale = 3.0
of = [{ input = "hour", curve = [[0, 1.0], [4, 1.0], [5, 0.0], [20, 0.0], [21, 1.0], [24, 1.0]] }]

[[sky_body]]
id = "green_moon"
field = "core:daylight"
term = "green_moon"
rise = 20.0
set = 5.0
peak = 70.0
arc = [0.0, 180.0]
color = "#7dffa0"
"##;

#[test]
fn a_mod_adds_a_moon_with_data_alone() {
    let dir = common::test_mods("sky-bodies", &["core"], &[("moons", &[("defs/sky.toml", GREEN_MOON)])]);
    let sim = Sim::build(&dir, 1, &|_| true, 32).unwrap();
    assert!(sim.warnings.is_empty(), "{:?}", sim.warnings);
    let bodies: Vec<&str> = sim.world.defs.sky_bodies.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(bodies, ["core:sun", "core:moon", "moons:green_moon"]);
    let green = &sim.world.defs.sky_bodies[2];
    assert_eq!((green.rgb, green.angular_size, green.shadows), ([0x7d, 0xff, 0xa0], 0.5, true), "defaults");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_sky_body_names_a_field_and_a_term_it_has() {
    for (bad, says) in [
        (GREEN_MOON.replace("term = \"green_moon\"", "term = \"blue_moon\""), "has no term 'blue_moon'"),
        (GREEN_MOON.replace("field = \"core:daylight\"\nterm", "field = \"moonshine\"\nterm"), "unknown field"),
        (GREEN_MOON.replace("peak = 70.0", "peak = 120.0"), "peak of 0 to 90"),
    ] {
        let dir = common::test_mods("sky-bodies-bad", &["core"], &[("moons", &[("defs/sky.toml", bad.as_str())])]);
        let err = Sim::build(&dir, 1, &|_| true, 32).err().expect("it doesn't load");
        assert!(err.contains(says), "{says}: {err}");
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn a_lone_sun_beside_bodies_is_a_warning() {
    let patch = "[[patch]]\ntarget = \"sky/core:core\"\nset = { sun = { rise = 6.0, set = 18.0, peak = 40.0, arc = [0.0, 180.0] } }\n";
    let dir = common::test_mods("sky-bodies-sun", &["core"], &[("old", &[("defs/sky.toml", patch)])]);
    let sim = Sim::build(&dir, 1, &|_| true, 32).unwrap();
    assert!(sim.warnings.iter().any(|w| w.contains("`sun` is ignored")), "{:?}", sim.warnings);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn cores_moon_lights_the_picture_and_not_the_sim() {
    let sim = Sim::build(&common::mods(), 1, &|id| id == "core", 32).unwrap();
    let defs = &sim.world.defs;
    let moon = defs.sky_bodies.iter().find(|b| b.id == "core:moon").expect("core has a moon");
    assert!(matches!(moon.light, rim_sim::defs::BodyLight::Own(_)), "its own terms, not a field's");
    let daylight = defs.lookup("field", "daylight").unwrap() as usize;
    let terms: Vec<&str> = defs.fields[daylight].terms.terms.iter().map(|t| t.label.as_str()).collect();
    assert_eq!(terms, ["sun"], "daylight is the sun's alone");
}

#[test]
fn a_bodys_brightness_is_a_fields_term_or_its_own_not_both() {
    let both = GREEN_MOON.replace("color = ", "of = [1.0]\ncolor = ");
    let neither = GREEN_MOON.replace("field = \"core:daylight\"\nterm = \"green_moon\"\n", "");
    for bad in [both, neither] {
        let dir = common::test_mods("sky-bodies-light", &["core"], &[("moons", &[("defs/sky.toml", bad.as_str())])]);
        let err = Sim::build(&dir, 1, &|_| true, 32).err().expect("it doesn't load");
        assert!(err.contains("its brightness is a `field`"), "{err}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
