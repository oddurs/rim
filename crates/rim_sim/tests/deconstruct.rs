//! Deconstruct: take a built thing down and get some of it back, in what
//! it was made of.

use rim_sim::defs::{DefId, Targets};
use rim_sim::hecs::Entity;
use rim_sim::order;
use rim_sim::world::{Blueprint, Designated, Job, Pawn, Thing};
use rim_sim::{Command, IVec, Sim};
use std::path::Path;

fn sim() -> (Sim, Entity) {
    let mut s = Sim::new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods"), 21).expect("mods load");
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

#[test]
fn right_click_offers_it_on_what_we_built() {
    let (mut s, founder) = sim();
    let (wall, stone) = (thing(&s, "wall"), thing(&s, "stone"));
    let at = open_cells(&s, 1)[0];
    let w = built(&mut s, wall, stone, at);
    let o = order::resolve(&s.world, founder, at, None).expect("an order on our wall");
    assert_eq!(o.label, "Deconstruct wall");
    assert!(matches!(o.job, Job::Deconstruct { target, .. } if target == w));

    // Giving the order marks it, so a later Cancel can still stop it.
    s.push(Command::Order { pawn: founder, cell: at, on: None });
    s.step();
    assert!(designated(&s, w), "an ordered deconstruct carries the mark");
    assert_eq!(order::job_text(&s.world, &s.world.ecs.get::<&Pawn>(founder).unwrap()), "Deconstruct wall");
}

#[test]
fn a_tree_still_offers_chop_not_deconstruct() {
    let (s, founder) = sim();
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
    if let Some(o) = order::resolve(&s.world, founder, tree, None) {
        assert!(o.label.starts_with("Chop"), "{}", o.label);
    }
}
