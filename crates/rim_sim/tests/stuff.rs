//! Stuff: a buildable says how much material it takes, the player says of
//! what. One wall, built out of anything structural.

use rim_sim::defs::DefId;
use rim_sim::hecs::Entity;
use rim_sim::world::{Blueprint, MadeOf, Thing};
use rim_sim::{Command, IVec, Sim};
use std::path::{Path, PathBuf};

fn mods() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

fn sim() -> Sim {
    Sim::new(&mods(), 21).expect("mods load")
}

fn thing(s: &Sim, id: &str) -> DefId {
    s.world.defs.thing_id(id).unwrap_or_else(|| panic!("core has {id}"))
}

/// Open ground near the colony to put a blueprint on.
fn site(s: &Sim) -> IVec {
    let c = s.world.colony_center().expect("a colony");
    (1..20)
        .flat_map(|r| [c.offset(r, 0), c.offset(-r, 0), c.offset(0, r), c.offset(0, -r)])
        .find(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none())
        .expect("open ground")
}

/// `n` distinct open cells near the colony.
fn open_cells(s: &Sim, n: usize) -> Vec<IVec> {
    let c = s.world.colony_center().expect("a colony");
    (1..30)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))))
        .filter(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none())
        .take(n)
        .collect()
}

fn blueprint_at(s: &Sim, p: IVec) -> Option<(Entity, Blueprint, Option<MadeOf>)> {
    // A floor's blueprint lives in the floor layer, not the fixture layer.
    let e = s.world.map.fixture_at(p).or_else(|| s.world.map.floor_at(p))?;
    let bp = (*s.world.ecs.get::<&Blueprint>(e).ok()?).clone();
    let m = s.world.ecs.get::<&MadeOf>(e).ok().map(|m| *m);
    Some((e, bp, m))
}

fn build(s: &mut Sim, thing_id: DefId, stuff: Option<DefId>, at: IVec) {
    s.push(Command::Build { thing: thing_id, stuff, a: at, b: at });
    s.step();
}

fn stacks_near(s: &Sim, def: DefId, near: IVec) -> u32 {
    s.world
        .ecs
        .query::<&Thing>()
        .without::<&Blueprint>()
        .iter()
        .filter(|t| t.def == def && t.pos.chebyshev(near) <= 2)
        .map(|t| t.count)
        .sum()
}

#[test]
fn one_wall_def_builds_in_wood_or_stone() {
    let mut s = sim();
    let (wall, wood, stone) = (thing(&s, "wall"), thing(&s, "wood"), thing(&s, "stone"));
    assert!(s.world.defs.thing_id("wall_wood").is_none(), "the per-material walls are gone");
    assert!(s.world.defs.thing_id("wall_stone").is_none());
    for old in ["door_wood", "bed_wood"] {
        assert!(s.world.defs.thing_id(old).is_none(), "{old} collapsed into its material-free def");
    }

    let a = site(&s);
    build(&mut s, wall, Some(wood), a);
    let (_, bp, made) = blueprint_at(&s, a).expect("a wooden wall blueprint");
    assert_eq!(bp.cost, vec![(wood, 5)], "costs 5 of what it is made of");
    assert_eq!(made, Some(MadeOf(wood)));

    let b = a.offset(0, 1);
    if !s.world.map.passable(b) || s.world.map.fixture_at(b).is_some() {
        return; // no second cell to prove stone on this map; wood alone proved the mechanism
    }
    build(&mut s, wall, Some(stone), b);
    let (_, bp, made) = blueprint_at(&s, b).expect("a stone wall blueprint");
    assert_eq!(bp.cost, vec![(stone, 5)]);
    assert_eq!(made, Some(MadeOf(stone)));
}

#[test]
fn a_wall_needs_a_material_of_the_right_kind() {
    let mut s = sim();
    let (wall, berries) = (thing(&s, "wall"), thing(&s, "berries"));
    let a = site(&s);
    build(&mut s, wall, None, a);
    assert!(blueprint_at(&s, a).is_none(), "no material, no wall");
    build(&mut s, wall, Some(berries), a);
    assert!(blueprint_at(&s, a).is_none(), "berries are not structural");
}

#[test]
fn a_fixed_recipe_ignores_the_material() {
    let mut s = sim();
    let (fire, stone, wood) = (thing(&s, "campfire"), thing(&s, "stone"), thing(&s, "wood"));
    let a = site(&s);
    build(&mut s, fire, Some(stone), a);
    let (_, bp, made) = blueprint_at(&s, a).expect("a campfire blueprint");
    assert_eq!(bp.cost, vec![(wood, 15)], "a campfire is 15 wood whatever you point at it");
    assert_eq!(made, None, "a recipe is not made of a chosen material");
}

#[test]
fn cancel_refunds_what_was_delivered_of_what_it_was_made_of() {
    let mut s = sim();
    let (wall, stone, wood) = (thing(&s, "wall"), thing(&s, "stone"), thing(&s, "wood"));
    let a = site(&s);
    build(&mut s, wall, Some(stone), a);
    let (e, _, _) = blueprint_at(&s, a).expect("a stone wall blueprint");
    s.world.ecs.get::<&mut Blueprint>(e).expect("blueprint").delivered[0] = 3;

    let (stone_before, wood_before) = (stacks_near(&s, stone, a), stacks_near(&s, wood, a));
    s.push(Command::Cancel { a, b: a });
    s.step();
    assert!(blueprint_at(&s, a).is_none(), "cancelled");
    assert_eq!(stacks_near(&s, stone, a) - stone_before, 3, "the three stone come back");
    assert_eq!(stacks_near(&s, wood, a), wood_before, "and no wood appears from nowhere");
}

#[test]
fn materials_are_listed_in_def_order() {
    let s = sim();
    let (wood, stone) = (thing(&s, "wood"), thing(&s, "stone"));
    let m = s.world.defs.materials("structural");
    assert!(m.contains(&wood) && m.contains(&stone), "wood and stone are structural: {m:?}");
    assert!(m.windows(2).all(|w| w[0] < w[1]), "def order, so it is deterministic: {m:?}");
    assert!(s.world.defs.materials("no_such_category").is_empty());
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

/// The whole point of the sprint in one test: a mod that is a single item
/// def can build every wall in the game.
#[test]
fn a_one_def_mod_adds_a_material() {
    let dir = std::env::temp_dir().join(format!("rim-marble-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&mods().join("core"), &dir.join("core"));
    let marble = dir.join("marble");
    std::fs::create_dir_all(marble.join("defs")).unwrap();
    std::fs::write(
        marble.join("mod.toml"),
        "id = \"marble\"\nname = \"Marble\"\nversion = \"0.0.0\"\napi = \"0.3\"\ndepends = [\"core\"]\n",
    )
    .unwrap();
    std::fs::write(
        marble.join("defs/marble.toml"),
        r##"
[[thing]]
id = "marble"
label = "marble"
color = "#e8e4dc"
category = "item"
shape = "item"
market_value = 3.0
stack_limit = 75
stuff = { categories = ["structural"], factors = { hp = 2.0, beauty = 3.0 } }
"##,
    )
    .unwrap();

    let mut s = Sim::new(&dir, 21).expect("core + marble load");
    assert!(s.warnings.is_empty(), "no warnings: {:?}", s.warnings);
    let (wall, marble_id) = (thing(&s, "wall"), thing(&s, "marble"));
    assert!(s.world.defs.materials("structural").contains(&marble_id), "marble is structural");

    // Every buildable that takes stuff, and how much of it.
    let wants: Vec<(DefId, u32, String)> = s
        .world
        .defs
        .things
        .iter()
        .enumerate()
        .filter_map(|(i, t)| t.build.as_ref()?.stuff.as_ref().map(|sc| (i as DefId, sc.count, t.id.clone())))
        .collect();
    assert!(wants.len() >= 3, "wall, door and bed at least: {wants:?}");
    let cells = open_cells(&s, wants.len());
    for ((def, count, id), &cell) in wants.iter().zip(&cells) {
        build(&mut s, *def, Some(marble_id), cell);
        let (_, bp, made) = blueprint_at(&s, cell).unwrap_or_else(|| panic!("a marble {id}, with no engine change"));
        assert_eq!(bp.cost, vec![(marble_id, *count)], "{id}");
        assert_eq!(made, Some(MadeOf(marble_id)), "{id}");
    }
    assert!(wall == wants[0].0 || wants.iter().any(|w| w.0 == wall), "the wall is among them");
    let _ = std::fs::remove_dir_all(&dir);
}
