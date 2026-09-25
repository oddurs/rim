//! The founder: the start's damage bonus, and a death that is a colony
//! event rather than the colony's end.

mod common;

use common::test_mods;
use rim_sim::hecs::Entity;
use rim_sim::world::{Faction, Pawn};
use rim_sim::{ai, Sim};
use std::fs;
use std::path::Path;

fn mods() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

#[test]
fn the_founder_swings_harder_by_the_starts_bonus() {
    let s = Sim::new(&mods(), 3).unwrap();
    let defs = &s.world.defs;
    let bonus = defs.start.as_ref().unwrap().founder_damage_bonus;
    assert!(bonus > 0, "core's start gives the founder a bonus");
    let founder = s.world.colonists().next().unwrap();
    let mut p: Pawn = (*s.world.ecs.get::<&Pawn>(founder).unwrap()).clone();
    assert!(p.founder);
    // At melee level 5 a swing is worth its creature's damage: pin it, so
    // the random starting level doesn't decide the test.
    let melee = defs.melee_skill.unwrap();
    p.skills.retain(|k| k.0 != melee);
    p.learn(melee, rim_sim::world::skill_xp(5));
    let plain = Pawn { founder: false, ..p.clone() };
    let base = defs.creature(p.def).melee_damage;
    assert_eq!(ai::melee_base(defs, &plain), base);
    assert_eq!(ai::melee_base(defs, &p), base + bonus);
    let (lo, hi) = ai::melee_bounds(defs, &p);
    let (plo, phi) = ai::melee_bounds(defs, &plain);
    assert!(lo > plo && hi > phi, "every roll is better: {lo}..{hi} against {plo}..{phi}");
}

fn kill(s: &mut Sim, e: Entity) {
    let mut p = s.world.ecs.get::<&mut Pawn>(e).unwrap();
    p.hp = 0;
    p.dead = true;
}

#[test]
fn a_founders_death_is_a_colony_event_and_not_the_end() {
    let script = r#"
rim.on("pawn_died", function(ev)
    if ev.founder then
        rim.set_data("watch:fell", ev.name)
    end
end)
"#;
    let dir = test_mods("founder", &["core"], &[("watch", &[("scripts/watch.luau", script)])]);
    let mut s = Sim::new(&dir, 5).unwrap();
    let founder = s.world.colonists().next().unwrap();
    let name = s.world.ecs.get::<&Pawn>(founder).unwrap().name.clone();
    // Another colonist, so the colony has someone left.
    let human = s.world.defs.creature_id("human").unwrap();
    let at = s.world.pawn_pos(founder).unwrap();
    s.world.spawn_pawn(human, Faction::Player, at.offset(1, 0), Some("Second".into()));
    for _ in 0..10 {
        s.step();
    }
    kill(&mut s, founder);
    for _ in 0..5 {
        s.step();
    }
    let told =
        s.world.messages.iter().any(|m| m.text == format!("{name}, the founder, has fallen. The colony goes on."));
    assert!(told, "the message names the founder: {:?}", s.world.messages.iter().map(|m| &m.text).collect::<Vec<_>>());
    assert!(
        s.world.recent_events.iter().any(|(_, kind, _, n)| *kind == "founder_died" && *n == name),
        "{:?}",
        s.world.recent_events
    );
    assert!(!s.world.colony_lost, "someone is left");
    let heard = matches!(s.world.data.get("watch:fell"), Some(rim_sim::data::Data::Str(n)) if *n == name);
    assert!(heard, "a mod heard pawn_died with founder = true: {:?}", s.world.data.get("watch:fell"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn the_founder_alone_is_still_the_colony() {
    let mut s = Sim::new(&mods(), 7).unwrap();
    let founder = s.world.colonists().next().unwrap();
    for _ in 0..10 {
        s.step();
    }
    kill(&mut s, founder);
    for _ in 0..5 {
        s.step();
    }
    assert!(s.world.colony_lost, "no one left: the colony is lost, as before");
}
