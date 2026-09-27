//! A build takes a material and parts, and may require a tool (DESIGN.md
//! §4f): a plank wall is planks and nails, and planks want a saw.

mod common;

use rim_sim::defs::DefId;
use rim_sim::world::{Blueprint, MadeOf, Thing};
use rim_sim::{Command, IVec, Sim};
use std::path::PathBuf;

const DEFS: &str = r##"
[[thing]]
id = "nail"
label = "nails"
color = "#8d98a3"
category = "item"
stack_limit = 200

[[thing]]
id = "saw"
label = "saw"
color = "#6f7c89"
category = "item"
hp = 60
tool = { tags = ["sawing"] }

[[thing]]
id = "plank_wall"
label = "plank wall"
color = "#b98a55"
category = "building"
blocks = true
build = { menu = "structure", work = 40, stuff = { category = "structural", count = 2 }, cost = [{ thing = "nail", count = 1 }] }

[[thing]]
id = "sawn_post"
label = "sawn post"
color = "#b98a55"
category = "building"
build = { menu = "structure", work = 40, requires = ["sawing"], cost = [{ thing = "core:wood", count = 1 }] }
"##;

fn world(name: &str) -> (Sim, PathBuf) {
    let dir = common::test_mods(name, &["core"], &[("parts", &[("defs/things.toml", DEFS)])]);
    let sim = Sim::with_mods(&dir, 21, &|_| true).expect("mods load");
    (sim, dir)
}

fn def(s: &Sim, id: &str) -> DefId {
    s.world.defs.thing_id(id).unwrap_or_else(|| panic!("no {id}"))
}

/// Open ground near the colony, clear of the founder's start items.
fn site(s: &Sim) -> IVec {
    let c = s.world.colony_center().expect("a colony");
    (4..30)
        .flat_map(|r| [c.offset(r, 0), c.offset(-r, 0), c.offset(0, r), c.offset(0, -r)])
        .find(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none())
        .expect("open ground")
}

/// Whether a finished `thing` stands at `p`.
fn built(s: &Sim, thing: DefId, p: IVec) -> bool {
    s.world
        .map
        .fixture_at(p)
        .is_some_and(|f| s.world.ecs.get::<&Blueprint>(f).is_err() && s.world.thing(f).is_some_and(|t| t.def == thing))
}

fn run(s: &mut Sim, ticks: u32) {
    for _ in 0..ticks {
        s.step();
    }
}

fn near(s: &Sim, d: DefId, p: IVec) -> u32 {
    s.world.ecs.query::<&Thing>().iter().filter(|t| t.def == d && t.pos.chebyshev(p) <= 3).map(|t| t.count).sum()
}

#[test]
fn a_material_and_parts_build_only_once_both_are_in() {
    let (mut s, dir) = world("parts-both");
    let (wall, wood, nail) = (def(&s, "parts:plank_wall"), def(&s, "wood"), def(&s, "parts:nail"));
    let p = site(&s);
    s.push(Command::Build { thing: wall, stuff: Some(wood), a: p, b: p, facing: 0 });
    s.step();
    let bp = s.world.map.fixture_at(p).expect("a plan");
    assert_eq!(
        s.world.ecs.get::<&Blueprint>(bp).unwrap().cost,
        vec![(wood, 2), (nail, 1)],
        "material first, then parts"
    );
    s.world.place_item(wood, p.offset(-2, 0), 10);
    run(&mut s, 4_000);
    assert!(!built(&s, wall, p), "no nails, no wall");
    assert_eq!(s.world.ecs.get::<&Blueprint>(bp).unwrap().delivered, vec![2, 0], "the wood is in");
    s.world.place_item(nail, p.offset(-2, 1), 5);
    run(&mut s, 4_000);
    assert!(built(&s, wall, p), "built once the nails came");
    let f = s.world.map.fixture_at(p).unwrap();
    assert_eq!(s.world.ecs.get::<&MadeOf>(f).map(|m| *m).ok(), Some(MadeOf(wood)), "made of its material");
    assert_eq!(s.world.cost_of(f), Some(vec![(wood, 2), (nail, 1)]), "taking it down gives back both");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn cancelling_a_half_delivered_plan_refunds_the_material_and_the_parts() {
    let (mut s, dir) = world("parts-cancel");
    let (wall, wood, nail) = (def(&s, "parts:plank_wall"), def(&s, "wood"), def(&s, "parts:nail"));
    let p = site(&s);
    s.push(Command::Build { thing: wall, stuff: Some(wood), a: p, b: p, facing: 0 });
    s.step();
    let bp = s.world.map.fixture_at(p).expect("a plan");
    s.world.ecs.get::<&mut Blueprint>(bp).unwrap().delivered = vec![1, 1];
    let (wood0, nail0) = (near(&s, wood, p), near(&s, nail, p));
    s.push(Command::Cancel { a: p, b: p });
    s.step();
    assert!(s.world.thing(bp).is_none(), "the plan is gone");
    assert_eq!((near(&s, wood, p) - wood0, near(&s, nail, p) - nail0), (1, 1), "what was brought comes back");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_build_that_needs_a_tool_waits_for_one_and_says_so() {
    let (mut s, dir) = world("parts-tool");
    let (post, wood, saw) = (def(&s, "parts:sawn_post"), def(&s, "wood"), def(&s, "parts:saw"));
    let p = site(&s);
    s.push(Command::Build { thing: post, stuff: None, a: p, b: p, facing: 0 });
    s.step();
    let bp = s.world.map.fixture_at(p).expect("a plan");
    s.world.place_item(wood, p.offset(-2, 0), 5);
    run(&mut s, 3_000);
    assert!(!built(&s, post, p), "nobody can saw");
    assert_eq!(s.world.ecs.get::<&Blueprint>(bp).unwrap().delivered, vec![1], "the wood is in");
    assert_eq!(rim_sim::ai::work_blocked(&s.world, bp).as_deref(), Some("Needs a sawing tool."));
    s.world.place_item(saw, p.offset(-3, 1), 1);
    run(&mut s, 4_000);
    assert!(built(&s, post, p), "built once a saw lay about");
    let _ = std::fs::remove_dir_all(dir);
}
