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

/// Growing and spoiling divide by their days in fixed point: days too few
/// to be one step of it fail the load, not a plant or spoil pass later.
#[test]
fn grow_and_spoil_days_are_at_least_one_step() {
    for (name, target, set) in [
        ("spoil-tiny", "thing/core:berries", "spoil = { days = 0.00001 }"),
        ("grow-tiny", "thing/core:berry_bush", "grow = { days = 0.00004 }"),
    ] {
        let patch = format!("[[patch]]\ntarget = \"{target}\"\nset = {{ {set} }}\n");
        let err = load_error(name, &[("defs/p.toml", &patch)]);
        assert!(err.contains("`days` of 0.0001 or more"), "{set}: {err}");
    }
}
