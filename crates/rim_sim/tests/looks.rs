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
    let samples: Vec<&str> = guide.split("```toml\n").skip(1).map(|b| b.split("```").next().unwrap()).collect();
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
