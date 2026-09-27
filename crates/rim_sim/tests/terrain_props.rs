//! Terrain properties and tags, read by terms: `terrain = prop` and
//! `near = tag`, with the distance grid patched where terrain changes.

mod common;

use rim_sim::data::Data;
use rim_sim::terms::NEAR_CAP;
use rim_sim::{IVec, Sim};

const SALT: &str = r##"
[[terrain]]
id = "salt_flat"
label = "salt flat"
color = "#e0dccc"
props = { salinity = 0.9, fertility = 0.1 }

[[field]]
id = "brine"
label = "brine"
kind = "derived"
range = [0.0, 100.0]
color_low = "#000000"
color_high = "#ffffff"

[field.value.salt]
scale = 100.0
of = [{ terrain = "salinity" }]

[field.value.acid]
of = [{ terrain = "acidity" }]

[[field]]
id = "damp"
label = "damp"
kind = "derived"
range = [0.0, 1.0]
color_low = "#000000"
color_high = "#ffffff"

[field.value.water]
of = [{ near = "water", curve = [[0, 1.0], [4, 0.0]] }]
"##;

const PROBE: &str = r#"
local M = {}
function M.fertility(x, y)
    return rim.terrain_prop(x, y, "fertility") + rim.terrain_prop(x, y, "nonsense")
end
return M
"#;

fn sim(name: &str) -> Sim {
    let files: &[(&str, &str)] = &[("defs/salt.toml", SALT), ("scripts/probe.luau", PROBE)];
    let dir = common::test_mods(name, &["core"], &[("salt", files)]);
    Sim::new(&dir, 7).expect("mods load")
}

fn value(s: &Sim, id: &str, p: IVec) -> f64 {
    let f = s.world.defs.lookup("field", id).unwrap() as usize;
    s.world.fields.value(&s.world.defs, &s.world.map, f, p)
}

fn water_tag(s: &Sim) -> usize {
    s.world.defs.terrain_tags.iter().position(|t| t == "water").unwrap()
}

/// The distance to the nearest water, the slow way.
fn brute(s: &Sim, p: IVec) -> u8 {
    let m = &s.world.map;
    let water = |q: IVec| s.world.defs.terrain[m.terrain[m.idx(q)] as usize].tags.iter().any(|t| t == "water");
    let r = NEAR_CAP as i32;
    let mut best = NEAR_CAP;
    for dy in -r..=r {
        for dx in -r..=r {
            let q = p.offset(dx, dy);
            if m.inb(q) && water(q) {
                best = best.min(dx.abs().max(dy.abs()) as u8);
            }
        }
    }
    best
}

fn paint(s: &mut Sim, id: &str, p: IVec) {
    let t = s.world.defs.lookup("terrain", id).unwrap();
    s.world.map.set_terrain(p, t, 100);
}

#[test]
fn a_mod_adds_a_prop_and_reads_it_in_terms() {
    let mut s = sim("terrain-prop");
    assert!(
        s.warnings.iter().any(|w| w.contains("field/salt:brine, value, term 'acid'") && w.contains("'acidity'")),
        "an unknown property is a load warning: {:?}",
        s.warnings
    );
    let d = &s.world.defs;
    assert_eq!(d.terrain_props, ["drainage", "fertility", "mud", "salinity", "water_table"]);
    let at = s.world.colony_center().unwrap().offset(6, 6);
    paint(&mut s, "salt:salt_flat", at);
    assert_eq!(value(&s, "brine", at), 90.0);
    // Core's grass gives no salinity: it reads 0.
    paint(&mut s, "grass", at);
    assert_eq!(value(&s, "brine", at), 0.0);
    assert_eq!(value(&s, "core:fertility", at), 100.0);
    paint(&mut s, "rich_soil", at);
    assert_eq!(value(&s, "core:fertility", at), 140.0);
}

#[test]
fn near_reads_the_distance_to_water_and_follows_terrain_changes() {
    let mut s = sim("terrain-near");
    s.step();
    let tag = water_tag(&s);
    let o = s.world.colony_center().unwrap().offset(-20, -20);
    // Dry ground well past the cap all round, then a pond at the middle.
    let r = NEAR_CAP as i32;
    for y in -r..40 + r {
        for x in -r..40 + r {
            let p = o.offset(x, y);
            if s.world.map.inb(p) {
                paint(&mut s, "grass", p);
            }
        }
    }
    let pond = o.offset(20, 20);
    paint(&mut s, "shallow_water", pond);
    s.world.map.ensure_near();
    let m = &s.world.map;
    assert_eq!(m.near(tag, m.idx(pond)), 0);
    assert_eq!(m.near(tag, m.idx(pond.offset(3, -2))), 3);
    assert_eq!(m.near(tag, m.idx(pond.offset(19, 0))), NEAR_CAP);
    // The term reads it: water 2 cells away is half damp.
    assert_eq!(value(&s, "damp", pond.offset(2, 1)), 0.5);

    // Dig two more ponds and fill the first: only the windows around the
    // changes are searched, and every distance matches the slow way.
    let before = s.world.map.near_patched();
    let changes = [(pond, "grass"), (o.offset(5, 30), "shallow_water"), (o.offset(33, 8), "deep_water")];
    for (p, t) in changes {
        paint(&mut s, t, p);
    }
    s.step();
    let searched = s.world.map.near_patched() - before;
    let window = (4 * NEAR_CAP as u64 + 1).pow(2);
    assert!(searched > 0 && searched <= changes.len() as u64 * window, "searched {searched} cells");
    for y in 0..40 {
        for x in 0..40 {
            let p = o.offset(x, y);
            let m = &s.world.map;
            assert_eq!(m.near(tag, m.idx(p)), brute(&s, p), "at {p:?}");
        }
    }
}

#[test]
fn scripts_read_terrain_props() {
    let mut s = sim("terrain-script");
    let at = s.world.colony_center().unwrap().offset(4, 4);
    paint(&mut s, "sand", at);
    let args = [Some(Data::Int(at.x as i64)), Some(Data::Int(at.y as i64))];
    let got = s.scripts.call_export(&mut s.world, "@salt/scripts/probe", "fertility", &args).unwrap();
    assert_eq!(got, Some(Data::Num(0.2)));
}
