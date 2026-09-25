//! Material factors: the def says the base, the material scales it, and the
//! engine never learns what any of the names mean.

use rim_sim::defs::DefId;
use rim_sim::hecs::Entity;
use rim_sim::world::{Blueprint, Thing, Work};
use rim_sim::{IVec, Sim};
use std::path::{Path, PathBuf};

fn mods() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

fn sim() -> Sim {
    Sim::new(&mods(), 21).expect("mods load")
}

fn thing(s: &Sim, id: &str) -> DefId {
    s.world.defs.thing_id(id).unwrap_or_else(|| panic!("defs have {id}"))
}

fn open_cells(s: &Sim, n: usize) -> Vec<IVec> {
    let c = s.world.colony_center().expect("a colony");
    (1..30)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))))
        .filter(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none())
        .take(n)
        .collect()
}

fn blueprint(s: &mut Sim, def: DefId, stuff: DefId, at: IVec) -> Entity {
    s.world.spawn_fixture_of(def, at, true, Some(stuff)).expect("blueprint placed")
}

/// Hand-deliver everything and finish it, without waiting for a pawn.
fn finish(s: &mut Sim, e: Entity) {
    {
        let mut bp = s.world.ecs.get::<&mut Blueprint>(e).expect("blueprint");
        let need: Vec<u32> = bp.cost.iter().map(|c| c.1).collect();
        bp.delivered = need;
    }
    rim_sim::ai::complete_building(&mut s.world, e);
}

#[test]
fn a_stone_wall_is_tougher_and_slower_than_a_wooden_one() {
    let mut s = sim();
    let (wall, wood, stone) = (thing(&s, "wall"), thing(&s, "wood"), thing(&s, "stone"));
    let cells = open_cells(&s, 2);
    let w = blueprint(&mut s, wall, wood, cells[0]);
    let st = blueprint(&mut s, wall, stone, cells[1]);
    let hp = |s: &Sim, e| s.world.ecs.get::<&Thing>(e).unwrap().hp;
    let work = |s: &Sim, e| s.world.ecs.get::<&Work>(e).unwrap().total;
    assert!(hp(&s, st) > hp(&s, w), "stone {} vs wood {} hp", hp(&s, st), hp(&s, w));
    assert!(work(&s, st) > work(&s, w), "stone {} vs wood {} work", work(&s, st), work(&s, w));
    // And the numbers come from nowhere but the defs.
    let base = s.world.defs.thing(wall);
    let f = |m, n| s.world.defs.factor(Some(m), n);
    assert_eq!(hp(&s, st), (base.hp as f64 * f(stone, "hp")).round() as i32);
    assert_eq!(work(&s, w), (base.build.as_ref().unwrap().work as f64 * f(wood, "work")).round() as u32);
}

#[test]
fn stat_reads_base_times_factor_and_bare_factors_alone() {
    let mut s = sim();
    let (wall, stone, fire) = (thing(&s, "wall"), thing(&s, "stone"), thing(&s, "campfire"));
    let cells = open_cells(&s, 2);
    let st = blueprint(&mut s, wall, stone, cells[0]);
    finish(&mut s, st);
    let base = s.world.defs.thing(wall);
    let f = |n| s.world.defs.factor(Some(stone), n);
    assert_eq!(s.world.stat(st, "hp"), Some(base.hp as f64 * f("hp")));
    assert_eq!(s.world.stat(st, "value"), Some(base.market_value * f("value")));
    // A name the engine has no base for is the material's number, verbatim.
    // (Spelled in pieces: the_engine_does_not_know_the_word greps for it.)
    let word = ["insul", "ation"].concat();
    assert_eq!(s.world.stat(st, &word), s.world.defs.factor_declared(Some(stone), &word));
    assert!(s.world.stat(st, &word).is_some(), "core's stone declares it");
    assert_eq!(s.world.stat(st, "no_such_stat"), None, "nobody declared it");
    // A recipe thing has no material: base alone.
    let fp = s.world.spawn_fixture(fire, cells[1], false).expect("a campfire");
    assert_eq!(s.world.stat(fp, "hp"), Some(s.world.defs.thing(fire).hp as f64));
}

#[test]
fn wealth_counts_what_a_wall_is_made_of() {
    let mut s = sim();
    let (wall, wood, stone) = (thing(&s, "wall"), thing(&s, "wood"), thing(&s, "stone"));
    let cells = open_cells(&s, 2);
    let base = s.world.defs.thing(wall).market_value;
    let before = {
        rim_sim::systems::wealth(&mut s.world);
        s.world.wealth
    };
    let w = blueprint(&mut s, wall, wood, cells[0]);
    finish(&mut s, w);
    rim_sim::systems::wealth(&mut s.world);
    let with_wood = s.world.wealth - before;
    let st = blueprint(&mut s, wall, stone, cells[1]);
    finish(&mut s, st);
    rim_sim::systems::wealth(&mut s.world);
    let with_stone = s.world.wealth - before - with_wood;
    let f = |m| s.world.defs.factor(Some(m), "value");
    assert!((with_wood - base * f(wood)).abs() < 1e-6, "wood wall worth {with_wood}");
    assert!((with_stone - base * f(stone)).abs() < 1e-6, "stone wall worth {with_stone}");
}

/// The acceptance criterion: no engine code names a material property.
/// Walks every crate's `src/` -- tests may talk about content, the engine
/// may not. The word is assembled here so this file is not its own hit.
#[test]
fn the_engine_does_not_know_the_word() {
    let word = ["insul", "ation"].concat();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut hits = Vec::new();
    fn walk(dir: &Path, word: &str, hits: &mut Vec<String>, in_src: bool) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name == "target" || name == "tests" || name == "examples" {
                    continue;
                }
                walk(&p, word, hits, in_src || name == "src");
            } else if in_src && p.extension().is_some_and(|x| x == "rs") {
                let text = std::fs::read_to_string(&p).unwrap_or_default().to_lowercase();
                if text.contains(word) {
                    hits.push(p.display().to_string());
                }
            }
        }
    }
    walk(&root, &word, &mut hits, false);
    assert!(hits.is_empty(), "engine code names a material property: {hits:?}");
}

// ---------------------------------------------------------- the real test

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

/// A mod declares a factor the engine has never heard of, builds a wall of
/// that material, and reads its own number back off the finished wall.
#[test]
fn a_factor_nobody_in_the_engine_knows_reaches_a_script() {
    let dir = std::env::temp_dir().join(format!("rim-sparkle-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&mods().join("core"), &dir.join("core"));
    let m = dir.join("glitter");
    std::fs::create_dir_all(m.join("defs")).unwrap();
    std::fs::create_dir_all(m.join("scripts")).unwrap();
    std::fs::write(
        m.join("mod.toml"),
        "id = \"glitter\"\nname = \"Glitter\"\nversion = \"0.0.0\"\napi = \"0.4\"\ndepends = [\"core\"]\n",
    )
    .unwrap();
    std::fs::write(
        m.join("defs/glitter.toml"),
        r##"
[[thing]]
id = "glitter"
label = "glitter"
color = "#f0e0ff"
category = "item"
market_value = 5.0
stack_limit = 75
stuff = { categories = ["structural"], factors = { sparkle = 7.0, hp = 2.0 } }
"##,
    )
    .unwrap();
    std::fs::write(
        m.join("scripts/glitter.luau"),
        r#"
rim.on("building_complete", function(ev)
	rim.message(`probe sparkle={rim.stat(ev.id, "sparkle")} hp={rim.stat(ev.id, "hp")} nothing={rim.stat(ev.id, "nothing")}`)
end)
"#,
    )
    .unwrap();

    let mut s = Sim::new(&dir, 21).expect("core + glitter load");
    assert!(s.warnings.is_empty(), "warnings: {:?}", s.warnings);
    let (wall, glitter) = (thing(&s, "wall"), thing(&s, "glitter"));
    let at = open_cells(&s, 1)[0];
    let e = blueprint(&mut s, wall, glitter, at);
    finish(&mut s, e);
    s.step(); // events reach scripts on the next tick
    let texts: Vec<_> = s.world.messages.iter().map(|m| m.text.clone()).collect();
    let hp = s.world.defs.thing(wall).hp * 2;
    let want = format!("probe sparkle=7 hp={hp} nothing=nil");
    assert!(texts.iter().any(|t| t == &want), "want {want:?} in {texts:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
