//! Looks: how things are drawn, as data (docs/modding/looks.md).

mod common;

use rim_sim::look::Prim;
use rim_sim::Sim;
use std::fs;

fn with_defs(name: &str, defs: &str) -> Result<Sim, String> {
    let dir = common::test_mods(name, &["core"], &[("probe", &[("defs/probe.toml", defs)])]);
    Sim::new(&dir, 1)
}

/// Every TOML sample in docs/modding/looks.md loads, in one mod beside core.
#[test]
fn guide_samples_load() {
    let guide = fs::read_to_string(common::mods().join("../docs/modding/looks.md")).unwrap().replace("\r\n", "\n");
    // Blocks after `<!-- not a sample -->` are fragments, not whole defs.
    let samples: Vec<&str> = guide
        .split("```toml\n")
        .collect::<Vec<_>>()
        .windows(2)
        .filter(|w| !w[0].trim_end().ends_with("<!-- not a sample -->"))
        .map(|w| w[1].split("```").next().unwrap())
        .collect();
    assert!(samples.len() >= 2, "found {} samples", samples.len());
    let s =
        with_defs("looks-guide", &samples.join("\n")).unwrap_or_else(|e| panic!("the guide's samples don't load: {e}"));
    let d = &s.world.defs;
    let fence = d.thing(d.thing_id("fence").unwrap());
    assert_eq!(d.join_groups[fence.look_r.join.unwrap() as usize], "fence");
    assert!(matches!(fence.look_r.layers[1].prim, Prim::Edges { width } if width == 2.0));
}

#[test]
fn a_shape_is_an_error_that_says_what_replaced_it() {
    let e = with_defs(
        "looks-shape",
        "[[thing]]\nid = \"loom\"\nlabel = \"loom\"\ncolor = \"#886644\"\ncategory = \"building\"\nshape = \"table\"\n",
    )
    .err()
    .expect("a def with a shape doesn't load");
    assert!(e.contains("thing/probe:loom") && e.contains("replaced by `look`"), "{e}");
}

#[test]
fn a_bad_layer_names_the_def_and_the_layer() {
    let e = with_defs(
        "looks-bad",
        "[[thing]]\nid = \"loom\"\nlabel = \"loom\"\ncolor = \"#886644\"\ncategory = \"building\"\nlook.layers = [{ draw = \"fill\" }, { draw = \"disc\", w = 0.5 }]\n",
    )
    .err()
    .expect("a bad layer doesn't load");
    assert!(e.contains("thing/probe:loom") && e.contains("look.layers[2]") && e.contains("`w` does not apply"), "{e}");
}

/// Core's walls, windows and doors join one another and nothing else does.
#[test]
fn core_wall_runs_join() {
    let s = Sim::new(&common::mods(), 1).unwrap();
    let d = &s.world.defs;
    let group = |id: &str| d.thing(d.thing_id(id).unwrap()).look_r.join;
    assert!(group("wall").is_some());
    assert_eq!(group("wall"), group("window"));
    assert_eq!(group("wall"), group("door"));
    assert_eq!(group("table"), None);
}

/// A sprite key names a PNG in a mod's `sprites/`: bare for the def's own
/// mod, prefixed for another's. One that has no file doesn't load.
#[test]
fn sprite_keys_find_their_files_or_fail_naming_them() {
    let def = |sprite: &str| {
        format!(
            "[[thing]]\nid = \"loom\"\nlabel = \"loom\"\ncolor = \"#886644\"\ncategory = \"building\"\nlook.layers = [{{ draw = \"sprite\", sprite = \"{sprite}\" }}]\n"
        )
    };
    // test_mods writes text files, so the PNG is copied in after.
    let dir = common::test_mods("looks-sprite", &["core"], &[("probe", &[("defs/probe.toml", &def("loom"))])]);
    std::fs::create_dir_all(dir.join("probe/sprites")).unwrap();
    std::fs::copy(common::mods().join("wildlife_plus/sprites/salt_lick.png"), dir.join("probe/sprites/loom.png"))
        .unwrap();
    let s = Sim::new(&dir, 1).expect("a sprite with a file loads");
    let d = &s.world.defs;
    assert_eq!(d.sprites, ["probe:loom"]);
    assert!(d.sprite_files[0].ends_with("probe/sprites/loom.png"), "{:?}", d.sprite_files);

    let e = with_defs("looks-sprite-missing", &def("core:nothing")).err().expect("a missing sprite doesn't load");
    assert!(
        e.contains("thing/probe:loom")
            && e.contains("unknown sprite 'core:nothing'")
            && e.contains("core/sprites/nothing.png"),
        "{e}"
    );
    let e =
        with_defs("looks-sprite-nomod", &def("elsewhere:x")).err().expect("a sprite from no loaded mod doesn't load");
    assert!(e.contains("no mod 'elsewhere' is loaded"), "{e}");
    for bad in ["../core/sprites/x", "a//b", "m:/etc/x", "a\\\\b"] {
        let e = with_defs("looks-sprite-path", &def(bad)).err().expect("a name outside sprites/ doesn't load");
        assert!(e.contains("a path inside the mod's sprites/"), "{bad}: {e}");
    }
}

#[test]
fn a_glyph_that_is_not_one_character_names_the_def() {
    let e = with_defs(
        "looks-glyph",
        "[[thing]]\nid = \"stone\"\nlabel = \"stone\"\ncolor = \"#888888\"\ncategory = \"building\"\nlook.layers = [{ draw = \"glyph\", glyph = \"ab\" }]\n",
    )
    .err()
    .expect("a two-character glyph doesn't load");
    assert!(e.contains("thing/probe:stone") && e.contains("one character"), "{e}");
    let s = with_defs(
        "looks-glyph-ok",
        "[[thing]]\nid = \"stone\"\nlabel = \"stone\"\ncolor = \"#888888\"\ncategory = \"building\"\nlook.layers = [{ draw = \"glyph\", glyph = \"\u{16b1}\" }]\n",
    )
    .unwrap();
    assert!(s.world.defs.glyphs.contains(&"\u{16b1}".to_string()));
}
