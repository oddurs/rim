//! Right-click orders. The pawn does the thing the cursor named, once,
//! and then goes back to choosing its own work.

use rim_sim::hecs::Entity;
use rim_sim::order;
use rim_sim::world::{Blueprint, Faction, Job, Pawn, Thing};
use rim_sim::{Command, IVec, Sim};
use std::path::Path;

fn sim(seed: u64) -> (Sim, Entity) {
    let mut s = Sim::new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods"), seed).expect("mods load");
    s.step();
    let founder = s.world.colonists().next().unwrap();
    (s, founder)
}

fn run(s: &mut Sim, ticks: u32) {
    for _ in 0..ticks {
        s.step();
    }
}

/// Step until `done`, up to `cap` ticks. An undrafted pawn picks its own
/// work the instant an order ends, so arrival has to be caught as it happens.
fn run_until(s: &mut Sim, cap: u32, done: impl Fn(&Sim) -> bool) -> bool {
    for _ in 0..cap {
        if done(s) {
            return true;
        }
        s.step();
    }
    done(s)
}

fn at(s: &Sim, e: Entity) -> IVec {
    s.world.pawn_pos(e).expect("pawn alive")
}

fn job(s: &Sim, e: Entity) -> Job {
    s.world.ecs.get::<&Pawn>(e).unwrap().job.clone()
}

/// Cells in rings around `from`, nearest first.
fn around(from: IVec, max: i32) -> impl Iterator<Item = IVec> {
    (1..=max).flat_map(move |r| {
        (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| from.offset(dx, dy))).filter(move |p| p.chebyshev(from) == r)
    })
}

/// The nearest reachable thing of `def` that the founder could be sent to.
fn nearest_thing(s: &mut Sim, from: IVec, def: &str) -> (Entity, IVec) {
    let id = s.world.defs.thing_id(def).unwrap();
    s.world.map.ensure_regions();
    let mut best: Option<(u32, Entity, IVec)> = None;
    for (e, t) in s.world.ecs.query::<&Thing>().without::<&Blueprint>().iter() {
        if t.def != id {
            continue;
        }
        let d = t.pos.octile(from);
        if best.is_none_or(|b| d < b.0) && s.world.map.can_reach(from, rim_sim::path::Goal::Touch(t.pos)) {
            best = Some((d, e, t.pos));
        }
    }
    let (_, e, p) = best.unwrap_or_else(|| panic!("no reachable {def} on the map"));
    (e, p)
}

fn order(s: &mut Sim, pawn: Entity, cell: IVec, on: Option<Entity>) {
    s.push(Command::Order { pawn, cell, on });
    s.step();
}

#[test]
fn order_chops_a_tree_nobody_designated() {
    let (mut s, founder) = sim(3);
    let here = at(&s, founder);
    let (tree, tp) = nearest_thing(&mut s, here, "tree_oak");
    assert!(s.world.ecs.get::<&rim_sim::world::Designated>(tree).is_err(), "tree starts undesignated");

    let hint = order::resolve(&s.world, founder, tp, None).expect("a tree offers an order");
    assert_eq!(hint.label, "Chop oak tree");
    order(&mut s, founder, tp, None);
    assert!(matches!(job(&s, founder), Job::Harvest { target, .. } if target == tree), "ordered to chop");

    // 280 work ticks plus the walk.
    run(&mut s, 3000);
    assert!(s.world.thing(tree).is_none(), "the tree is gone");
    let wood = s.world.defs.thing_id("wood").unwrap();
    let dropped = s.world.ecs.query::<&Thing>().iter().any(|(_, t)| t.def == wood && t.pos.chebyshev(tp) <= 2);
    assert!(dropped, "chopping leaves wood behind");
}

#[test]
fn order_moves_an_undrafted_colonist() {
    let (mut s, founder) = sim(4);
    let here = at(&s, founder);
    s.world.map.ensure_regions();
    let region = s.world.map.region_at(here);
    let dest = around(here, 12)
        .find(|&p| p.chebyshev(here) >= 6 && s.world.map.passable(p) && s.world.map.region_at(p) == region)
        .expect("somewhere to walk");

    assert!(!s.world.ecs.get::<&Pawn>(founder).unwrap().drafted, "undrafted");
    assert_eq!(order::resolve(&s.world, founder, dest, None).unwrap().label, "Go here");
    order(&mut s, founder, dest, None);
    assert!(matches!(job(&s, founder), Job::MoveTo { to } if to == dest), "ordered to move");
    assert!(run_until(&mut s, 900, |s| at(s, founder) == dest), "the colonist walks there");
}

#[test]
fn order_hauls_to_a_blueprint_then_builds_it() {
    let (mut s, founder) = sim(5);
    let here = at(&s, founder);
    let (wall, wood) = (s.world.defs.thing_id("wall_wood").unwrap(), s.world.defs.thing_id("wood").unwrap());
    let site = around(here, 10)
        .find(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none())
        .expect("room to build");
    let bp = s.world.spawn_fixture(wall, site, true).expect("blueprint placed");
    s.world.place_item(wood, here, 40);
    s.step();

    let hint = order::resolve(&s.world, founder, site, None).expect("a blueprint offers an order");
    assert_eq!(hint.label, "Haul wood to wooden wall");
    order(&mut s, founder, site, None);
    assert!(matches!(job(&s, founder), Job::Deliver { bp: b, .. } if b == bp), "ordered to haul");

    run(&mut s, 4000);
    assert!(s.world.ecs.get::<&Blueprint>(bp).is_err(), "the wall finishes: {:?}", job(&s, founder));
}

#[test]
fn order_hunts_a_wild_animal() {
    let (mut s, founder) = sim(6);
    let here = at(&s, founder);
    s.world.map.ensure_regions();
    let region = s.world.map.region_at(here);
    let spot = around(here, 8)
        .find(|&p| p.chebyshev(here) >= 3 && s.world.map.passable(p) && s.world.map.region_at(p) == region)
        .expect("room for a deer");
    let deer = s.world.spawn_pawn(s.world.defs.creature_id("deer").unwrap(), Faction::Wild, spot, None);
    s.step();

    let hint = order::resolve(&s.world, founder, spot, Some(deer)).expect("an animal offers an order");
    assert_eq!(hint.label, "Hunt deer");
    order(&mut s, founder, spot, Some(deer));
    assert!(matches!(job(&s, founder), Job::Attack { target, .. } if target == deer), "ordered to hunt");
}

#[test]
fn a_drafted_pawn_only_moves_and_fights() {
    let (mut s, founder) = sim(3);
    let here = at(&s, founder);
    let (_, tp) = nearest_thing(&mut s, here, "tree_oak");
    s.push(Command::Draft { pawn: founder, on: true });
    s.step();

    // A soldier does not take work: the click is a move, or nothing at all
    // when the tree's own cell cannot be stood in.
    if let Some(o) = order::resolve(&s.world, founder, tp, None) {
        assert!(matches!(o.job, Job::MoveTo { .. }), "drafted pawns do not work: {}", o.label);
    }
}

#[test]
fn an_order_takes_work_off_the_pawn_already_doing_it() {
    let (mut s, founder) = sim(3);
    let here = at(&s, founder);
    let (tree, tp) = nearest_thing(&mut s, here, "tree_oak");
    let second = s.world.spawn_pawn(s.world.defs.creature_id("human").unwrap(), Faction::Player, here, None);
    s.step();

    order(&mut s, founder, tp, None);
    assert!(matches!(job(&s, founder), Job::Harvest { target, .. } if target == tree));

    order(&mut s, second, tp, None);
    assert!(matches!(job(&s, second), Job::Harvest { target, .. } if target == tree), "the second pawn takes it");
    assert!(!matches!(job(&s, founder), Job::Harvest { .. }), "the first pawn is off the job");
    assert!(!s.world.reserved_by_other(tree, second), "the claim moved across");
}

#[test]
fn an_order_ends_and_the_pawn_goes_back_to_its_own_work() {
    let (mut s, founder) = sim(4);
    let here = at(&s, founder);
    s.world.map.ensure_regions();
    let region = s.world.map.region_at(here);
    let dest = around(here, 12)
        .find(|&p| p.chebyshev(here) >= 6 && s.world.map.passable(p) && s.world.map.region_at(p) == region)
        .expect("somewhere to walk");
    order(&mut s, founder, dest, None);
    assert!(run_until(&mut s, 900, |s| at(s, founder) == dest), "arrives");
    // The order was one job, not a standing state: the pawn picks its own
    // work again rather than holding position the way a drafted one would.
    assert!(run_until(&mut s, 120, |s| !matches!(job(s, founder), Job::MoveTo { .. })), "the move order is spent");
}
