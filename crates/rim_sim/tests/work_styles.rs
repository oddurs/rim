//! Work styles and grow windows: how work on a cell looks, as data
//! (DESIGN.md §6b, docs/modding/looks.md).

mod common;

use rim_sim::defs::{Exit, Strike, Wear};
use rim_sim::Sim;

fn with_defs(name: &str, defs: &str) -> Result<Sim, String> {
    let dir = common::test_mods(name, &["core"], &[("probe", &[("defs/probe.toml", defs)])]);
    Sim::new(&dir, 1)
}

#[test]
fn core_names_a_style_for_each_kind_of_work() {
    let s = Sim::new(&common::mods(), 1).unwrap();
    let d = &s.world.defs;
    let style_of = |thing: &str| {
        let h = d.thing(d.thing_id(thing).unwrap()).harvest.iter().find(|h| h.destroy).unwrap();
        let st = d.designations[h.desig_r as usize].style_r.expect("a style");
        &d.work_styles[st as usize]
    };
    let chop = style_of("tree_oak");
    assert_eq!((chop.wear, chop.exit), (Wear::Lean, Exit::Fall));
    assert!(chop.strike.contains(&Strike::Shed));
    assert_eq!(style_of("granite").wear, Wear::Cracks);
    let builds = &d.work_styles[d.build_style.expect("core says how builds look") as usize];
    assert_eq!(builds.wear, Wear::Grow);
}

#[test]
fn an_unknown_effect_is_an_error_that_lists_the_real_ones() {
    let e = with_defs("styles-unknown", "[[work_style]]\nid = \"sawing\"\nevery = 10\nstrike = [\"sparks\"]\n")
        .err()
        .expect("an unknown strike doesn't load");
    assert!(e.contains("sawing") && e.contains("sparks") && e.contains("shake"), "{e}");
}

#[test]
fn every_must_be_at_least_one() {
    let e = with_defs("styles-every", "[[work_style]]\nid = \"idle\"\nevery = 0\n").err().expect("refused");
    assert!(e.contains("work_style/probe:idle") && e.contains("every"), "{e}");
}

#[test]
fn only_one_style_is_the_one_builds_use() {
    let e = with_defs("styles-builds", "[[work_style]]\nid = \"masonry\"\nevery = 10\nbuilds = true\n")
        .err()
        .expect("a second builds style is refused");
    assert!(e.contains("work_style/probe:masonry") && e.contains("core:building"), "{e}");
}

/// Rule 3: a thing that blocks must keep its outline until it is gone, so
/// the style that takes it down may crack it but not shrink it.
#[test]
fn a_blocking_thing_cannot_be_taken_down_by_shrinking() {
    let patch = "[[work_style]]\nid = \"shrink\"\nevery = 10\nwear = \"grow\"\n\n\
                 [[patch]]\ntarget = \"designation/core:deconstruct\"\nset = { style = \"probe:shrink\" }\n";
    let e = with_defs("styles-rule3", patch).err().expect("refused");
    assert!(e.contains("work_style/probe:shrink") && e.contains("blocks"), "{e}");
}

#[test]
fn a_grow_window_must_be_inside_the_work() {
    let thing = |w: &str| {
        format!(
            "[[thing]]\nid = \"loom\"\nlabel = \"loom\"\ncolor = \"#886644\"\ncategory = \"building\"\n\
             look.layers = [{{ draw = \"fill\", grow = {w} }}]\n"
        )
    };
    assert!(with_defs("grow-ok", &thing("[0.2, 0.6]")).is_ok());
    for bad in ["[0.6, 0.2]", "[0.0, 1.5]", "[-0.1, 0.5]"] {
        let e = with_defs("grow-bad", &thing(bad)).err().unwrap_or_else(|| panic!("{bad} loads"));
        assert!(e.contains("thing/probe:loom") && e.contains("grow"), "{bad}: {e}");
    }
}

#[test]
fn layers_share_the_work_when_no_window_is_given() {
    let s = Sim::new(&common::mods(), 1).unwrap();
    let d = &s.world.defs;
    let oak = &d.thing(d.thing_id("tree_oak").unwrap()).look_r.layers;
    let n = oak.len();
    assert_eq!(oak[0].window(0, n), [0.0, 1.0 / n as f32]);
    assert_eq!(oak[n - 1].window(n - 1, n), [(n - 1) as f32 / n as f32, 1.0]);
    let wall = &d.thing(d.thing_id("wall").unwrap()).look_r.layers;
    assert_eq!(wall[1].window(1, 2), [0.85, 1.0], "the wall says its own");
}
