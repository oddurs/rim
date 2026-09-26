//! Deconstruct: take a built thing down and get some of it back, in what
//! it was made of.

use rim_sim::defs::{DefId, Targets};
use rim_sim::hecs::Entity;
use rim_sim::order;
use rim_sim::world::{Blueprint, Designated, Job, Pawn, Thing};
use rim_sim::{Command, IVec, Sim};
use std::path::Path;

fn sim() -> (Sim, Entity) {
    // Core's own rules: plugins may gate or add harvests (mods/primitive).
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let mut s = Sim::with_mods(&mods, 21, &|m| m == "core").expect("mods load");
    s.step();
    let founder = s.world.colonists().next().expect("a founder");
    // Alone, so nobody else takes the job.
    for e in s.world.pawns.clone() {
        if e != founder {
            let _ = s.world.ecs.despawn(e);
        }
    }
    s.world.pawns.retain(|&e| e == founder);
    (s, founder)
}

fn thing(s: &Sim, id: &str) -> DefId {
    s.world.defs.thing_id(id).unwrap_or_else(|| panic!("thing {id}"))
}

fn deconstruct_id(s: &Sim) -> DefId {
    s.world.defs.designations.iter().position(|d| d.targets == Targets::Built).expect("core offers deconstruct")
        as DefId
}

fn open_cells(s: &Sim, n: usize) -> Vec<IVec> {
    let c = s.world.colony_center().expect("a colony");
    (1..30)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))))
        .filter(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none())
        .take(n)
        .collect()
}

/// Build it the way the colony would: owned, made of `stuff`.
fn built(s: &mut Sim, def: DefId, stuff: DefId, at: IVec) -> Entity {
    let e = s.world.spawn_fixture_of(def, at, false, Some(stuff)).expect("placed");
    rim_sim::ai::complete_building(&mut s.world, e);
    e
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

fn designated(s: &Sim, e: Entity) -> bool {
    s.world.ecs.get::<&Designated>(e).is_ok()
}

#[test]
fn designating_marks_built_things_only() {
    let (mut s, _) = sim();
    let (wall, stone) = (thing(&s, "wall"), thing(&s, "stone"));
    let cells = open_cells(&s, 2);
    let w = built(&mut s, wall, stone, cells[0]);
    let bp = s.world.spawn_fixture_of(wall, cells[1], true, Some(stone)).expect("a blueprint");
    // A tree somewhere in the same box.
    let oak = thing(&s, "tree_oak");
    let tree = s
        .world
        .ecs
        .query::<(Entity, &Thing)>()
        .iter()
        .find(|(_, t)| t.def == oak)
        .map(|(e, _)| e)
        .expect("an oak on the map");
    let (lo, hi) = (IVec::new(0, 0), IVec::new(s.world.map.w - 1, s.world.map.h - 1));
    s.push(Command::Designate { designation: deconstruct_id(&s), a: lo, b: hi });
    s.step();
    assert!(designated(&s, w), "the wall is marked");
    assert!(!designated(&s, bp), "a blueprint is cancelled, not deconstructed");
    assert!(!designated(&s, tree), "a tree is not something we built");
}

#[test]
fn a_stone_wall_gives_stone_back() {
    let (mut s, founder) = sim();
    let (wall, stone, wood) = (thing(&s, "wall"), thing(&s, "stone"), thing(&s, "wood"));
    let at = open_cells(&s, 1)[0];
    let w = built(&mut s, wall, stone, at);
    let (stone0, wood0) = (stacks_near(&s, stone, at), stacks_near(&s, wood, at));
    s.push(Command::Designate { designation: deconstruct_id(&s), a: at, b: at });
    s.step();

    let mut took_job = false;
    for _ in 0..6000 {
        s.step();
        took_job |=
            matches!(s.world.ecs.get::<&Pawn>(founder).unwrap().job, Job::Deconstruct { target, .. } if target == w);
        if s.world.thing(w).is_none() {
            break;
        }
    }
    assert!(took_job, "the colonist took the job");
    assert!(s.world.thing(w).is_none(), "the wall is gone");
    let refund = s.world.defs.thing(wall).build.as_ref().unwrap().refund;
    let want = (5.0 * refund).round() as u32;
    assert_eq!(stacks_near(&s, stone, at) - stone0, want, "{refund} of five stone comes back");
    assert_eq!(stacks_near(&s, wood, at), wood0, "and no wood from nowhere");
}

#[test]
fn cancel_stops_it() {
    let (mut s, founder) = sim();
    let (wall, stone) = (thing(&s, "wall"), thing(&s, "stone"));
    let at = open_cells(&s, 1)[0];
    let w = built(&mut s, wall, stone, at);
    s.push(Command::Designate { designation: deconstruct_id(&s), a: at, b: at });
    s.step();
    for _ in 0..200 {
        s.step();
        if matches!(s.world.ecs.get::<&Pawn>(founder).unwrap().job, Job::Deconstruct { .. }) {
            break;
        }
    }
    assert!(matches!(s.world.ecs.get::<&Pawn>(founder).unwrap().job, Job::Deconstruct { .. }), "under way");
    s.push(Command::Cancel { a: at, b: at });
    s.step();
    s.step();
    assert!(!designated(&s, w), "unmarked");
    assert!(!matches!(s.world.ecs.get::<&Pawn>(founder).unwrap().job, Job::Deconstruct { .. }), "and dropped");
    assert!(s.world.thing(w).is_some(), "the wall stands");
}

/// The bug: a right-click meant as "go here" landed on our wall and took it
/// down. Now a plain right-click never deconstructs; the menu names it.
#[test]
fn a_plain_right_click_never_deconstructs_but_the_menu_can() {
    let (mut s, founder) = sim();
    let (wall, stone) = (thing(&s, "wall"), thing(&s, "stone"));
    let at = open_cells(&s, 1)[0];
    let w = built(&mut s, wall, stone, at);
    assert!(order::resolve(&s.world, founder, at, None).is_none(), "nothing safe to do at our wall");
    let options = order::options(&s.world, founder, at, None);
    let take_down = options.iter().find(|c| c.label == "Deconstruct wall").expect("the menu offers it");
    assert!(take_down.damaging, "and says it takes something away");
    let key = take_down.key.clone();

    // A plain order does nothing to it.
    s.push(Command::Order { pawn: founder, cell: at, on: None, pick: None });
    s.step();
    assert!(!designated(&s, w), "a plain right-click leaves the wall alone");
    assert!(!matches!(s.world.ecs.get::<&Pawn>(founder).unwrap().job, Job::Deconstruct { .. }));

    // Picked by name, it's given, and marked so a later Cancel can stop it.
    s.push(Command::Order { pawn: founder, cell: at, on: None, pick: Some(key) });
    s.step();
    assert!(designated(&s, w), "an ordered deconstruct carries the mark");
    assert_eq!(order::job_text(&s.world, &s.world.ecs.get::<&Pawn>(founder).unwrap()), "Deconstruct wall");
}

/// A tree offers its harvests, never a deconstruct; felling one nobody
/// marked is damaging, and once marked it's the plain click's order.
#[test]
fn a_tree_offers_chop_not_deconstruct() {
    let (mut s, founder) = sim();
    let oak = thing(&s, "tree_oak");
    let from = s.world.pawn_pos(founder).unwrap();
    let tree = s
        .world
        .ecs
        .query::<&Thing>()
        .iter()
        .filter(|t| t.def == oak)
        .min_by_key(|t| t.pos.octile(from))
        .map(|t| t.pos)
        .expect("an oak");
    let options = order::options(&s.world, founder, tree, None);
    assert!(options.iter().all(|c| !c.label.starts_with("Deconstruct")), "no deconstruct on a tree");
    if let Some(chop) = options.iter().find(|c| c.label.starts_with("Chop")) {
        assert!(chop.damaging, "felling an unmarked tree is damaging");
        let chop_def = s.world.defs.lookup("designation", "chop").unwrap();
        s.push(Command::Designate { designation: chop_def, a: tree, b: tree });
        s.step();
        let o = order::resolve(&s.world, founder, tree, None).expect("marked, a plain click chops");
        assert!(o.label.starts_with("Chop"), "{}", o.label);
    }
}

/// An order taken back: the pawn stops, the mark the order put on the wall
/// goes, and the wall stands.
#[test]
fn an_undone_deconstruct_leaves_the_wall_unmarked() {
    let (mut s, founder) = sim();
    let (wall, stone) = (thing(&s, "wall"), thing(&s, "stone"));
    let at = open_cells(&s, 1)[0];
    let w = built(&mut s, wall, stone, at);
    let key =
        order::options(&s.world, founder, at, None).into_iter().find(|c| c.label == "Deconstruct wall").unwrap().key;
    s.push(Command::Order { pawn: founder, cell: at, on: None, pick: Some(key) });
    s.step();
    assert!(designated(&s, w) && matches!(s.world.ecs.get::<&Pawn>(founder).unwrap().job, Job::Deconstruct { .. }));
    s.push(Command::UndoOrder { pawn: founder, target: Some(w), cell: at, unmark: Some(w) });
    s.step();
    assert!(!designated(&s, w), "the order's mark is gone");
    assert!(
        !matches!(s.world.ecs.get::<&Pawn>(founder).unwrap().job, Job::Deconstruct { .. }),
        "and the pawn is off it"
    );
    for _ in 0..600 {
        s.step();
    }
    assert!(s.world.thing(w).is_some(), "the wall stands");
}

/// Undo only stops the job the order gave: a pawn that has moved on to
/// something else keeps doing it.
#[test]
fn undo_leaves_a_pawn_that_moved_on_alone() {
    let (mut s, founder) = sim();
    let here = s.world.pawn_pos(founder).unwrap();
    let there = open_cells(&s, 1)[0];
    s.push(Command::Order { pawn: founder, cell: there, on: None, pick: Some("move".into()) });
    s.step();
    // Undoing a walk to somewhere else, which this pawn isn't doing.
    s.push(Command::UndoOrder { pawn: founder, target: None, cell: here.offset(40, 40), unmark: None });
    s.step();
    assert!(
        matches!(s.world.ecs.get::<&Pawn>(founder).unwrap().job, Job::MoveTo { to } if to == there),
        "still walking there"
    );
    s.push(Command::UndoOrder { pawn: founder, target: None, cell: there, unmark: None });
    s.step();
    assert!(
        !matches!(s.world.ecs.get::<&Pawn>(founder).unwrap().job, Job::MoveTo { to } if to == there),
        "the walk undone"
    );
}
