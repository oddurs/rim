//! The why panel: what a colonist would do, why it passes over the rest,
//! and who would take a job (DESIGN.md §4d).

mod common;

use rim_sim::ai::{explain_work, who_takes, Why, WorkWhy};
use rim_sim::hecs::Entity;
use rim_sim::world::{Pawn, Thing};
use rim_sim::{Command, IVec, Sim};

fn core() -> Sim {
    Sim::with_mods(&common::mods(), 2, &|m| m == "core").unwrap()
}

fn why_of(s: &Sim, pawn: Entity, work: &str) -> WorkWhy {
    let t = s.world.defs.lookup("work_type", work).unwrap();
    explain_work(&s.world, pawn).into_iter().find(|w| w.work == t).unwrap()
}

/// The oak nearest `at`.
fn oak_near(s: &Sim, at: IVec) -> (Entity, IVec) {
    let oak = s.world.defs.thing_id("tree_oak").unwrap();
    s.world
        .ecs
        .query::<(Entity, &Thing)>()
        .iter()
        .filter(|(_, t)| t.def == oak)
        .map(|(e, t)| (e, t.pos))
        .min_by_key(|(e, p)| (p.octile(at), e.id()))
        .unwrap()
}

fn designate(s: &mut Sim, id: &str, at: IVec) {
    let d = s.world.defs.lookup("designation", id).unwrap();
    s.push(Command::Designate { designation: d, a: at, b: at });
    s.step();
}

#[test]
fn never_nothing_and_picked() {
    let mut s = core();
    let pawn = s.world.colonists().next().unwrap();
    assert!(matches!(why_of(&s, pawn, "core:chop").why, Why::Nothing), "nothing marked to chop yet");
    let (_, at) = oak_near(&s, s.world.pawn_pos(pawn).unwrap());
    designate(&mut s, "chop", at);
    assert!(matches!(why_of(&s, pawn, "core:chop").why, Why::Picked(rim_sim::world::Job::Harvest { .. })));
    let chop = s.world.defs.lookup("work_type", "core:chop").unwrap();
    s.push(Command::SetPriority { pawn, work: chop, level: 0 });
    s.step();
    assert!(matches!(why_of(&s, pawn, "core:chop").why, Why::Never));
}

#[test]
fn a_better_level_beats_it() {
    let mut s = core();
    let pawn = s.world.colonists().next().unwrap();
    let c = s.world.pawn_pos(pawn).unwrap();
    let (_, at) = oak_near(&s, c);
    designate(&mut s, "chop", at);
    let (wall, wood) = (s.world.defs.thing_id("wall").unwrap(), s.world.defs.thing_id("wood").unwrap());
    s.world.place_item(wood, c.offset(1, 1), 20);
    s.push(Command::Build { thing: wall, stuff: Some(wood), a: c.offset(3, 3), b: c.offset(3, 3) });
    let build = s.world.defs.lookup("work_type", "core:build").unwrap();
    s.push(Command::SetPriority { pawn, work: build, level: 1 });
    s.step();
    let chop = why_of(&s, pawn, "core:chop");
    assert!(matches!(chop.why, Why::Beaten(b) if b == build), "{chop:?}");
    assert!(chop.dist.is_some(), "it knows how far the chop was");
}

#[test]
fn someone_else_has_it() {
    let mut s = core();
    let pawn = s.world.colonists().next().unwrap();
    let (tree, at) = oak_near(&s, s.world.pawn_pos(pawn).unwrap());
    designate(&mut s, "chop", at);
    // Every chop job is someone else's.
    let other = s.world.spawn_pawn(
        s.world.defs.creature_id("human").unwrap(),
        rim_sim::world::Faction::Player,
        at.offset(0, 2),
        None,
    );
    s.world.reserve(tree, other);
    assert!(matches!(why_of(&s, pawn, "core:chop").why, Why::Reserved));
}

#[test]
fn walled_in_is_unreachable() {
    let mut s = core();
    let pawn = s.world.colonists().next().unwrap();
    let (_, at) = oak_near(&s, s.world.pawn_pos(pawn).unwrap());
    designate(&mut s, "chop", at);
    let (wall, wood) = (s.world.defs.thing_id("wall").unwrap(), s.world.defs.thing_id("wood"));
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx == -2 || dx == 2 || dy == -2 || dy == 2 {
                let p = at.offset(dx, dy);
                if let Some(f) = s.world.map.fixture_at(p) {
                    s.world.despawn_thing(f);
                }
                s.world.spawn_fixture_of(wall, p, false, wood);
            }
        }
    }
    // The colonist outside the ring.
    let out = (4..20).map(|k| at.offset(k, 0)).find(|&p| s.world.map.passable(p)).unwrap();
    s.world.ecs.get::<&mut Pawn>(pawn).unwrap().pos = out;
    s.world.map.ensure_regions();
    assert!(!s.world.map.can_reach(out, rim_sim::path::Goal::Touch(at)));
    let why = why_of(&s, pawn, "core:chop");
    assert!(matches!(why.why, Why::Unreachable), "{why:?}");
}

#[test]
fn a_plan_with_nothing_to_build_it_from() {
    let mut s = core();
    let pawn = s.world.colonists().next().unwrap();
    let c = s.world.pawn_pos(pawn).unwrap();
    let (wall, wood) = (s.world.defs.thing_id("wall").unwrap(), s.world.defs.thing_id("wood").unwrap());
    let loose: Vec<Entity> =
        s.world.ecs.query::<(Entity, &Thing)>().iter().filter(|(_, t)| t.def == wood).map(|(e, _)| e).collect();
    for e in loose {
        s.world.despawn_thing(e);
    }
    s.push(Command::Build { thing: wall, stuff: Some(wood), a: c.offset(3, 3), b: c.offset(3, 3) });
    s.step();
    let why = why_of(&s, pawn, "core:build");
    assert!(matches!(why.why, Why::NoMaterials(Some(d)) if d == wood), "{why:?}");
}

#[test]
fn a_gated_harvest_names_its_tool() {
    // The stone age gates chopping an oak behind a chopping tool.
    let mut s = Sim::new(&common::mods(), 2).unwrap();
    let pawn = s.world.colonists().next().unwrap();
    let (_, at) = oak_near(&s, s.world.pawn_pos(pawn).unwrap());
    designate(&mut s, "chop", at);
    let why = why_of(&s, pawn, "core:chop");
    let Why::NeedsTool(mask) = why.why else { panic!("{why:?}") };
    assert_eq!(s.world.defs.tool_tag_names(mask), vec!["chopping"]);
}

/// Who would take a job is who does: the first of the ranking reserves it.
#[test]
fn the_first_in_line_takes_it() {
    let mut s = core();
    let founder = s.world.colonists().next().unwrap();
    let c = s.world.pawn_pos(founder).unwrap();
    let human = s.world.defs.creature_id("human").unwrap();
    for i in 0..3 {
        s.world.spawn_pawn(human, rim_sim::world::Faction::Player, c.offset(2 * i + 1, 1), None);
    }
    let (tree, at) = oak_near(&s, c.offset(4, 0));
    designate(&mut s, "chop", at);
    let line = who_takes(&s.world, tree);
    assert!(!line.is_empty(), "someone would take it");
    for _ in 0..600 {
        if s.world.reservations.contains_key(&tree) {
            break;
        }
        s.step();
    }
    assert_eq!(s.world.reservations.get(&tree), Some(&line[0].0), "the first in line took it");
    assert!(who_takes(&s.world, tree).is_empty(), "held, nobody is next for it");
}

/// Explaining doesn't change what a colonist does: it reserves nothing.
#[test]
fn explaining_changes_nothing() {
    let mut s = core();
    let pawn = s.world.colonists().next().unwrap();
    let (tree, at) = oak_near(&s, s.world.pawn_pos(pawn).unwrap());
    designate(&mut s, "chop", at);
    let before = s.world.state_hash();
    let _ = explain_work(&s.world, pawn);
    let _ = who_takes(&s.world, tree);
    assert_eq!(s.world.state_hash(), before);
    assert!(s.world.reservations.get(&tree).is_none_or(|&r| r == pawn));
    let _ = s.world.ecs.get::<&Pawn>(pawn).unwrap();
}
