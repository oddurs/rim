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
