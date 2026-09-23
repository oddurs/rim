//! Wildlife behaviour comes from creature def flags: `flees` runs from
//! attackers, `aggressive` hunts intelligent creatures, neither retaliates.

use rim_sim::hecs::Entity;
use rim_sim::world::{Faction, Job, Pawn, Thing};
use rim_sim::{Command, IVec, Sim};
use std::path::Path;

/// A world with only the founder in it, drafted so they stand still.
fn isolated(seed: u64, drafted: bool) -> (Sim, Entity) {
    let mut s = Sim::new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods"), seed).expect("mods load");
    let founder = s.world.colonists().next().unwrap();
    for e in s.world.pawns.clone() {
        if e != founder {
            let _ = s.world.ecs.despawn(e);
        }
    }
    s.world.pawns.retain(|&e| e == founder);
    if drafted {
        s.push(Command::Draft { pawn: founder, on: true });
        s.step();
    }
    (s, founder)
}

fn pos(s: &Sim, e: Entity) -> IVec {
    s.world.pawn_pos(e).expect("pawn alive")
}

/// Spawn a wild creature on open ground exactly `dist` cells from `near`.
fn spawn_near(s: &mut Sim, creature: &str, near: IVec, dist: i32) -> Entity {
    let def = s.world.defs.creature_id(creature).unwrap_or_else(|| panic!("no creature {creature}"));
    s.world.map.ensure_regions();
    let region = s.world.map.region_at(near);
    let spot = (-dist..=dist)
        .flat_map(|dy| (-dist..=dist).map(move |dx| near.offset(dx, dy)))
        .filter(|p| p.chebyshev(near) == dist)
        .find(|&p| s.world.map.passable(p) && s.world.map.region_at(p) == region)
        .expect("no open cell nearby");
    s.world.spawn_pawn(def, Faction::Wild, spot, None)
}

fn provoke(s: &mut Sim, victim: Entity, attacker: Entity) {
    s.world.ecs.get::<&mut Pawn>(victim).unwrap().last_attacker = Some(attacker);
}

fn run(s: &mut Sim, ticks: u32) {
    for _ in 0..ticks {
        s.step();
    }
}

#[test]
fn prey_flees_from_attacker() {
    let (mut s, founder) = isolated(11, true);
    let fp = pos(&s, founder);
    let deer = spawn_near(&mut s, "deer", fp, 2);
    provoke(&mut s, deer, founder);
    run(&mut s, 60);
    assert!(matches!(s.world.ecs.get::<&Pawn>(deer).unwrap().job, Job::Flee { .. }), "provoked deer should flee");
    run(&mut s, 300);
    assert!(pos(&s, deer).chebyshev(fp) >= 6, "deer only got {} cells away", pos(&s, deer).chebyshev(fp));
}

#[test]
fn prey_leaves_people_alone() {
    let (mut s, founder) = isolated(12, true);
    let fp = pos(&s, founder);
    spawn_near(&mut s, "deer", fp, 2);
    spawn_near(&mut s, "hare", fp, 3);
    run(&mut s, 1500);
    let p = s.world.ecs.get::<&Pawn>(founder).unwrap();
    assert!(p.last_attacker.is_none(), "prey attacked unprovoked");
}

#[test]
fn non_fleeing_animal_retaliates() {
    // The boar comes from the wildlife_plus plugin: neither `flees` nor `aggressive`.
    let (mut s, founder) = isolated(13, true);
    let fp = pos(&s, founder);
    let boar = spawn_near(&mut s, "boar", fp, 2);
    run(&mut s, 200);
    assert!(s.world.ecs.get::<&Pawn>(founder).unwrap().last_attacker.is_none(), "boar attacked unprovoked");
    provoke(&mut s, boar, founder);
    run(&mut s, 400);
    assert_eq!(s.world.ecs.get::<&Pawn>(founder).unwrap().last_attacker, Some(boar), "provoked boar should fight back");
}

#[test]
fn predator_hunts_nearby_people() {
    let (mut s, founder) = isolated(14, true);
    let fp = pos(&s, founder);
    let wolf = spawn_near(&mut s, "wolf", fp, 5);
    run(&mut s, 600);
    let p = s.world.ecs.get::<&Pawn>(founder).unwrap();
    assert_eq!(p.last_attacker, Some(wolf), "wolf within 8 cells should attack");
}

#[test]
fn hunt_designation_kills_and_butchers() {
    let (mut s, founder) = isolated(15, false);
    let fp = pos(&s, founder);
    let hare = spawn_near(&mut s, "hare", fp, 4);
    let hunt = s.world.defs.lookup("designation", "hunt").unwrap();
    let hp = pos(&s, hare);
    s.push(Command::Designate { designation: hunt, a: hp, b: hp });
    run(&mut s, 3000);
    assert!(!s.world.pawn_alive(hare), "designated hare survived the hunt");
    let meat = s.world.defs.thing_id("raw_meat").unwrap();
    let got: u32 = s.world.ecs.query::<&Thing>().iter().filter(|(_, t)| t.def == meat).map(|(_, t)| t.count).sum();
    assert!(got > 0, "no meat dropped");
}

#[test]
fn hunting_needs_a_butcherable_wild_target() {
    // Designating a colonist for hunting must do nothing.
    let (mut s, founder) = isolated(16, false);
    let hunt = s.world.defs.lookup("designation", "hunt").unwrap();
    let fp = pos(&s, founder);
    s.push(Command::Designate { designation: hunt, a: fp, b: fp });
    s.step();
    assert!(s.world.ecs.get::<&rim_sim::world::Designated>(founder).is_err());
}
