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

/// A start's items resolve with the rest of the defs, so a tool that only
/// loads mods sees a bad one, not just a new game (470f2bd5).
#[test]
fn a_start_item_nobody_defines_is_a_load_error() {
    let patch = "[[patch]]\ntarget = \"start/core:warrior\"\nset = { items = [{ thing = \"nope\", count = 1 }] }\n";
    let dir = test_mods("start-items", &["core"], &[("bad", &[("defs/p.toml", patch)])]);
    let loaded = rim_sim::modloader::load(&dir);
    let _ = fs::remove_dir_all(dir);
    let err = loaded.err().expect("a start item that isn't defined fails the load");
    assert!(err.contains("start/core:warrior") && err.contains("nope"), "{err}");
}

/// A second `[[start]]` replaced the first by load order, without a word.
#[test]
fn a_second_start_is_a_load_error() {
    let start = "[[start]]\nid = \"landing\"\ncreature = \"core:human\"\n";
    let err = load_error("two-starts", &[("defs/start.toml", start)]);
    assert!(err.contains("only one [[start]] may exist; patch start/core:warrior instead"), "{err}");
}
