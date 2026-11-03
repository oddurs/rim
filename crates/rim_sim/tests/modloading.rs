//! Load errors say exactly where the problem is.

mod common;

use common::test_mods;
use rim_sim::Sim;
use std::fs;

fn load_error(name: &str, files: &[(&str, &str)]) -> String {
    let dir = test_mods(name, &["core"], &[("bad", files)]);
    let err = Sim::new(&dir, 1).err().expect("the bad mod must not load");
    let _ = fs::remove_dir_all(dir);
    err
}

#[test]
fn a_bad_value_names_its_key_path() {
    let err = load_error(
        "badpath",
        &[(
            "defs/bad.toml",
            r##"
[[thing]]
id = "hut_kit"
label = "hut kit"
color = "#aa8844"
category = "item"
build = { work = 10, cost = [{ thing = "wood", count = "ten" }] }
"##,
        )],
    );
    assert!(err.contains("thing/bad:hut_kit") && err.contains("bad/defs/bad.toml"), "{err}");
    println!("{err}");
    assert!(err.contains("build.cost[0].count"), "names the key path: {err}");
}

#[test]
fn a_bad_patch_names_the_mod_that_made_it() {
    let err = load_error(
        "badpatch",
        &[("defs/p.toml", "[[patch]]\ntarget = \"thing/core:tree_oak\"\nset = { hp = \"lots\" }\n")],
    );
    println!("{err}");
    assert!(err.contains("at `hp`") && err.contains("patched by 'bad'"), "{err}");
}

/// Core names `gather` so plugins agree on it (DESIGN.md §5 rule 2), but
/// none of its own things can be gathered: core alone is unchanged.
#[test]
fn core_names_gather_and_uses_it_nowhere() {
    let s = Sim::with_mods(&common::mods(), 1, &|m| m == "core").unwrap();
    let gather = s.world.defs.lookup("designation", "core:gather").expect("core offers gather");
    assert_eq!(s.world.defs.designations[gather as usize].label, "Gather");
    let gathered: Vec<&str> =
        s.world.defs.things.iter().filter(|t| t.harvest_for(gather).is_some()).map(|t| t.id.as_str()).collect();
    assert!(gathered.is_empty(), "core things with a gather harvest: {gathered:?}");
}

/// A need's rates are divided by and added to a level: days of 0, or a
/// fraction outside 0..1, fail the load rather than a needs pass later.
#[test]
fn a_need_s_rates_are_checked_at_load() {
    for (name, set) in [
        ("need-recover", "recover_days = 0.0"),
        ("need-empty", "days_to_empty = -0.0"),
        ("need-seek", "seek_below = 2.0"),
        ("need-damage", "empty_damage_per_day = -1.0"),
    ] {
        let patch = format!("[[patch]]\ntarget = \"need/core:warmth\"\nset = {{ {set} }}\n");
        let err = load_error(name, &[("defs/p.toml", &patch)]);
        assert!(err.contains("need/core:warmth") && err.contains("recover_days"), "{set}: {err}");
    }
    for (name, target, set) in [
        ("food-inf", "thing/core:berries", "food = { nutrition = inf }"),
        ("bed-neg", "thing/core:bed", "bed = { rest_rate = -1.0 }"),
    ] {
        let patch = format!("[[patch]]\ntarget = \"{target}\"\nset = {{ {set} }}\n");
        let err = load_error(name, &[("defs/p.toml", &patch)]);
        assert!(err.contains("nutrition") && err.contains(target.trim_start_matches("thing/")), "{set}: {err}");
    }
}

/// A rate the load allows but that is extreme fills the need, and the
/// addition saturates rather than overflowing the level.
#[test]
fn an_extreme_need_rate_saturates() {
    let patch = "[[patch]]\ntarget = \"need/core:warmth\"\nset = { recover_days = 1e-12 }\n";
    let dir = test_mods("need-fast", &["core"], &[("fast", &[("defs/p.toml", patch)])]);
    let mut s = Sim::new(&dir, 1).expect("loads");
    let feels = s.world.defs.lookup("field", "core:feels_like").unwrap() as usize;
    s.world.fields.set_ambient(feels, Some(20.0));
    let warmth = s.world.defs.lookup("need", "core:warmth").unwrap();
    for _ in 0..3 {
        rim_sim::systems::needs(&mut s.world);
    }
    for e in s.world.colonists().collect::<Vec<_>>() {
        let p = s.world.ecs.get::<&rim_sim::world::Pawn>(e).unwrap();
        assert_eq!(p.need(warmth), Some(rim_sim::world::NEED_MAX));
    }
    let _ = fs::remove_dir_all(dir);
}
