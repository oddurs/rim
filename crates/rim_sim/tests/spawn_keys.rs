//! Map generation keys each spawning def's pattern by its id (c3d18fe7): a
//! def added elsewhere, with no spawn of its own, doesn't move anything.

mod common;

use rim_sim::world::Thing;
use rim_sim::{IVec, Sim};

const EXTRA: &str = r##"
[[thing]]
id = "marker"
label = "marker"
color = "#ffffff"
category = "item"
"##;

const SHRUB: &str = r##"
[[thing]]
id = "shrub"
label = "shrub"
color = "#448833"
category = "plant"
natural = true
spawn = { terrain = ["core:grass"], density = 0.05 }
"##;

fn shrubs(name: &str, extra: bool) -> Vec<IVec> {
    let mut mods: Vec<(&str, &[(&str, &str)])> = Vec::new();
    // "a_extra" loads before "b_shrub" (ties break by id), so its def comes
    // ahead of the shrub in the list.
    if extra {
        mods.push(("a_extra", &[("defs/extra.toml", EXTRA)]));
    }
    mods.push(("b_shrub", &[("defs/shrub.toml", SHRUB)]));
    let dir = common::test_mods(name, &["core"], &mods);
    let s = Sim::build(&dir, 3, &|_| true, 64).unwrap();
    let shrub = s.world.defs.thing_id("b_shrub:shrub").unwrap();
    let mut at: Vec<IVec> = s.world.ecs.query::<&Thing>().iter().filter(|t| t.def == shrub).map(|t| t.pos).collect();
    at.sort();
    let _ = std::fs::remove_dir_all(dir);
    at
}

#[test]
fn a_def_added_elsewhere_moves_no_spawn() {
    let alone = shrubs("spawn-keys-alone", false);
    assert!(!alone.is_empty(), "the shrub spawns");
    assert_eq!(shrubs("spawn-keys-extra", true), alone);
}
