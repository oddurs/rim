//! The levels below the surface are made from `[[stratum]]` defs (DESIGN.md
//! §6d): the deepest sets how far down the map goes, each fills its level in
//! patches, only the surface has a map edge, and a script may make a level
//! itself.

mod common;

use rim_sim::defs::DefId;
use rim_sim::{IVec, Sim};

fn terrain(s: &Sim, id: &str) -> DefId {
    s.world.defs.lookup("terrain", id).unwrap_or_else(|| panic!("terrain {id}"))
}

fn at(s: &Sim, p: IVec) -> DefId {
    s.world.map.terrain[s.world.map.idx(p)]
}

#[test]
fn core_goes_three_levels_down_and_each_is_its_own_rock() {
    let s = Sim::build(&common::mods(), 1, &|m| m == "core", 96).unwrap();
    assert_eq!(s.world.map.levels(), -3..=0);
    let (w, h) = (s.world.map.w, s.world.map.h);
    let bedrock = terrain(&s, "core:bedrock");
    for z in -3..=-1 {
        let mut kinds = std::collections::BTreeSet::new();
        for y in 0..h {
            for x in 0..w {
                let p = IVec::at(x, y, z);
                let edge = x == 0 || y == 0 || x == w - 1 || y == h - 1;
                assert_eq!(at(&s, p) == bedrock, edge, "bedrock rings level {z}, and only its edge: {p:?}");
                assert!(s.world.solid_at(p).is_some(), "every cell below is rock until dug: {p:?}");
                kinds.insert(at(&s, p));
            }
        }
        assert!(kinds.len() >= 3, "level {z} is made of more than one rock: {kinds:?}");
    }
    let (clay, soil) = (terrain(&s, "core:clay"), terrain(&s, "core:packed_earth"));
    let wet: Vec<DefId> =
        ["marsh", "shallow_water", "deep_water", "rich_soil"].iter().map(|t| terrain(&s, t)).collect();
    let mut first = std::collections::BTreeSet::new();
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let t = at(&s, IVec::at(x, y, -1));
            if t == clay {
                assert!(wet.contains(&at(&s, IVec::new(x, y))), "clay lies only under wet ground");
            }
            first.insert(t);
        }
    }
    assert!(first.contains(&soil));
}

#[test]
fn the_same_seed_makes_the_same_levels() {
    let a = Sim::build(&common::mods(), 7, &|m| m == "core", 64).unwrap();
    let b = Sim::build(&common::mods(), 7, &|m| m == "core", 64).unwrap();
    assert!(a.world.map.terrain == b.world.map.terrain);
    let c = Sim::build(&common::mods(), 8, &|m| m == "core", 64).unwrap();
    assert!(a.world.map.terrain != c.world.map.terrain, "another seed, other rock");
}

#[test]
fn the_stone_age_gates_the_levels_by_tool() {
    let s = Sim::new(&common::mods(), 1).unwrap();
    let d = &s.world.defs;
    let requires = |id: &str| d.thing(d.thing_id(id).unwrap()).harvest[0].requires.clone();
    assert_eq!(requires("core:packed_earth"), vec!["digging"]);
    assert_eq!(requires("core:clay_bed"), vec!["digging"]);
    assert_eq!(requires("core:limestone"), vec!["pounding"]);
    let core = Sim::build(&common::mods(), 1, &|m| m == "core", 64).unwrap();
    let d = &core.world.defs;
    for id in ["packed_earth", "clay_bed", "chalk", "limestone", "wet_limestone", "granite"] {
        assert!(d.thing(d.thing_id(id).unwrap()).harvest[0].requires.is_empty(), "core alone gates nothing: {id}");
    }
}

const DEEP: &str = r#"
-- Level -2 is all granite, with a band of chalk through the middle.
rim.on_generate_level(-2, function(z)
    local w, h = rim.map_size()
    for y = 1, h - 2 do
        for x = 1, w - 2 do
            local t = if y == h // 2 then "core:chalk" else "core:granite"
            rim.set_terrain(x, y, t, z)
        end
    end
    assert(rim.terrain_at(1, h // 2, z) == "core:chalk")
    local n = rim.noise(3, 4, 10)
    assert(n >= 0 and n <= 1 and n == rim.noise(3, 4, 10))
end)
"#;

#[test]
fn a_script_can_make_a_level() {
    let dir = common::test_mods("strata-script", &["core"], &[("deep", &[("scripts/deep.luau", DEEP)])]);
    let s = Sim::build(&dir, 3, &|_| true, 48).unwrap();
    let (granite, chalk, bedrock) =
        (terrain(&s, "core:granite"), terrain(&s, "core:chalk"), terrain(&s, "core:bedrock"));
    for y in 1..47 {
        for x in 1..47 {
            let want = if y == 24 { chalk } else { granite };
            assert_eq!(at(&s, IVec::at(x, y, -2)), want);
        }
    }
    assert_eq!(at(&s, IVec::at(0, 0, -2)), bedrock, "the stratum's edge still stands");
    assert_ne!(at(&s, IVec::at(5, 5, -1)), granite, "the other levels are the strata's");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_generator_that_fails_fails_the_new_game() {
    let dir = common::test_mods(
        "strata-broken",
        &["core"],
        &[("broken", &[("scripts/broken.luau", "rim.on_generate_level(-1, function(z) error(\"no\") end)")])],
    );
    let err = Sim::build(&dir, 3, &|_| true, 32).err().expect("fails");
    assert!(err.contains("generating level -1"), "{err}");
    let _ = std::fs::remove_dir_all(dir);
}
