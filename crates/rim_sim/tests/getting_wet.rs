//! Getting wet (DESIGN.md §4c): rain soaks a colonist outdoors, a soaked
//! one feels the cold sooner, and they dry out of the rain, faster warm.

mod common;

use rim_sim::hecs::Entity;
use rim_sim::systems::{self, NEEDS_INTERVAL};
use rim_sim::world::{Faction, Pawn, NEED_MAX};
use rim_sim::{Sim, TICKS_PER_DAY};

fn sim() -> (Sim, Entity) {
    let mut s = Sim::new(&common::mods(), 8).expect("mods load");
    s.step();
    let me = s.world.colonists().next().expect("a founder");
    for e in s.world.pawns.clone() {
        if e != me && s.world.ecs.get::<&Pawn>(e).is_ok_and(|p| p.faction == Faction::Player) {
            let _ = s.world.ecs.despawn(e);
        }
    }
    s.world.pawns.retain(|&e| s.world.ecs.get::<&Pawn>(e).is_ok());
    (s, me)
}

fn pin(s: &mut Sim, id: &str, v: f64) {
    let f = s.world.defs.lookup("field", id).unwrap() as usize;
    s.world.fields.set_ambient(f, Some(v));
}

/// Only the needs pass, for `hours`.
fn hours(s: &mut Sim, h: f64) {
    for _ in 0..(h * TICKS_PER_DAY as f64 / 24.0 / NEEDS_INTERVAL as f64) as u64 {
        systems::needs(&mut s.world);
    }
}

fn wet(s: &Sim, e: Entity) -> f64 {
    s.world.ecs.get::<&Pawn>(e).unwrap().wet as f64 / 10_000.0
}

fn warmth(s: &Sim, e: Entity) -> i32 {
    let d = s.world.defs.lookup("need", "core:warmth").unwrap();
    s.world.ecs.get::<&Pawn>(e).unwrap().needs.iter().find(|n| n.0 == d).unwrap().1
}

fn set_warmth(s: &mut Sim, e: Entity, v: i32) {
    let d = s.world.defs.lookup("need", "core:warmth").unwrap();
    s.world.ecs.get::<&mut Pawn>(e).unwrap().needs.iter_mut().filter(|n| n.0 == d).for_each(|n| n.1 = v);
}

#[test]
fn rain_soaks_a_colonist_who_then_feels_the_cold_until_dry() {
    let (mut s, me) = sim();
    assert!(!s.world.map.indoors(s.world.pawn_pos(me).unwrap()), "the founder starts out in the open");
    // An hour of steady rain at a mild 12°.
    pin(&mut s, "core:feels_like", 12.0);
    pin(&mut s, "core:precipitation", 4.0);
    hours(&mut s, 1.0);
    assert!(wet(&s, me) > 0.3, "soaked: {}", wet(&s, me));

    // The rain stops. At 12°, comfortable dry, a wet colonist still feels
    // the cold; a dry one in the same place doesn't.
    pin(&mut s, "core:precipitation", 0.0);
    let (mut dry, dme) = sim();
    pin(&mut dry, "core:feels_like", 12.0);
    for (sim, who) in [(&mut s, me), (&mut dry, dme)] {
        set_warmth(sim, who, NEED_MAX / 2);
        hours(sim, 0.5);
    }
    assert!(warmth(&s, me) < warmth(&dry, dme), "wet {} dry {}", warmth(&s, me), warmth(&dry, dme));

    // Out of the rain it dries within hours, faster by a fire.
    let (mut fire, fme) = sim();
    for (sim, who, t) in [(&mut s, me, 12.0), (&mut fire, fme, 30.0)] {
        pin(sim, "core:precipitation", 4.0);
        pin(sim, "core:feels_like", t);
        sim.world.ecs.get::<&mut Pawn>(who).unwrap().wet = 10_000;
        pin(sim, "core:precipitation", 0.0);
        hours(sim, 1.5);
    }
    assert!(wet(&fire, fme) < wet(&s, me), "fire {} mild {}", wet(&fire, fme), wet(&s, me));
    hours(&mut s, 3.0);
    assert_eq!(wet(&s, me), 0.0, "dry within four hours");
}
