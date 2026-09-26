//! Skills: colonists get better at what they do, and work and fight
//! faster or harder for it.

mod common;

use rim_sim::world::{skill_xp, Pawn, Thing};
use rim_sim::{Command, Sim};

#[test]
fn skills_load_and_work_types_name_them() {
    let s = Sim::new(&common::mods(), 1).unwrap();
    let d = &s.world.defs;
    let plants = d.lookup("skill", "core:plants").unwrap();
    let chop = d.lookup("work_type", "core:chop").unwrap();
    assert_eq!(d.work_types[chop as usize].skill_r, Some(plants));
    assert_eq!(d.melee_skill, d.lookup("skill", "core:melee"));
    let founder = s.world.colonists().next().unwrap();
    let p = s.world.ecs.get::<&Pawn>(founder).unwrap();
    assert_eq!(p.skills.len(), d.skills.len(), "a person arrives with every skill");
    assert!(p.skills.iter().all(|&(k, xp)| p.skill(k) <= 6 && xp == skill_xp(p.skill(k))));
}

#[test]
fn levels_follow_experience() {
    let mut p = Pawn::default();
    assert_eq!(p.skill(0), 0);
    p.learn(0, skill_xp(3));
    assert_eq!(p.skill(0), 3);
    p.learn(0, skill_xp(4) - skill_xp(3) - 1);
    assert_eq!(p.skill(0), 3, "one short of the next level");
    p.learn(0, u32::MAX);
    assert_eq!(p.skill(0), 20, "capped");
}

/// Ticks for the founder, at a plants level, to fell the nearest oak.
fn chop_ticks(level: u32) -> (u64, u32) {
    // Core alone: with the stone age on, the oak would wait for an axe.
    let mut s = Sim::with_mods(&common::mods(), 3, &|m| m == "core").unwrap();
    let defs = s.world.defs.clone();
    let founder = s.world.colonists().next().unwrap();
    let plants = defs.lookup("skill", "core:plants").unwrap();
    {
        let mut p = s.world.ecs.get::<&mut Pawn>(founder).unwrap();
        p.skills.retain(|k| k.0 != plants);
        p.learn(plants, skill_xp(level));
    }
    let oak = defs.thing_id("tree_oak").unwrap();
    let at = s.world.pawn_pos(founder).unwrap();
    let (tree, pos) = s
        .world
        .ecs
        .query::<(rim_sim::hecs::Entity, &Thing)>()
        .iter()
        .filter(|(_, t)| t.def == oak)
        .map(|(e, t)| (e, t.pos))
        .min_by_key(|(e, p)| (p.octile(at), e.id()))
        .unwrap();
    let chop = defs.lookup("designation", "chop").unwrap();
    s.push(Command::Designate { designation: chop, a: pos, b: pos });
    let start = s.world.tick;
    while s.world.thing(tree).is_some() && s.world.tick < start + 20_000 {
        s.step();
    }
    let xp = s.world.ecs.get::<&Pawn>(founder).unwrap().skills.iter().find(|k| k.0 == plants).unwrap().1;
    (s.world.tick - start, xp - skill_xp(level))
}

#[test]
fn a_skilled_hand_works_faster_and_every_job_teaches() {
    let (slow, learned) = chop_ticks(0);
    let (fast, _) = chop_ticks(12);
    assert!(fast < slow, "level 12 fells it in {fast} ticks, level 0 in {slow}");
    assert!(learned > 0, "chopping taught plants");
}

#[test]
fn melee_skill_makes_a_swing_worth_more() {
    let s = Sim::new(&common::mods(), 1).unwrap();
    let defs = &s.world.defs;
    let melee = defs.melee_skill.unwrap();
    let mut p: Pawn = (*s.world.ecs.get::<&Pawn>(s.world.colonists().next().unwrap()).unwrap()).clone();
    p.founder = false;
    p.skills.retain(|k| k.0 != melee);
    let untrained = rim_sim::ai::melee_base(defs, &p);
    p.learn(melee, skill_xp(20));
    let master = rim_sim::ai::melee_base(defs, &p);
    assert!(master > untrained, "master {master}, untrained {untrained}");
}

/// A colonist saved before skills existed loads knowing the middle of
/// what people arrive knowing, not nothing.
#[test]
fn a_person_without_skills_loads_with_middling_ones() {
    let s = Sim::new(&common::mods(), 2).unwrap();
    let founder = s.world.colonists().next().unwrap();
    s.world.ecs.get::<&mut Pawn>(founder).unwrap().skills.clear();
    let back = rim_sim::snapshot::Snapshot::capture(&s).restore(&common::mods(), &|_| true).unwrap();
    let p = back.world.ecs.get::<&Pawn>(founder).unwrap();
    assert_eq!(p.skills.len(), back.world.defs.skills.len());
    assert!(p.skills.iter().all(|&(k, _)| p.skill(k) == 3), "{:?}", p.skills);
}
