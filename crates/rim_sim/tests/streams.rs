//! Random streams per purpose (DESIGN.md §7b): a draw in one stream never
//! moves another's rolls.

mod common;

use common::test_mods;
use rim_sim::rng::{Streams, AI, SPAWNS};
use rim_sim::world::{Pawn, Thing};
use rim_sim::{Sim, TICKS_PER_DAY};
use std::fs;

/// What a game did, outside any one mod's data: every pawn and every thing.
fn outcome(s: &Sim) -> Vec<String> {
    let w = &s.world;
    let mut out: Vec<String> = w
        .pawns
        .iter()
        .filter_map(|&e| w.ecs.get::<&Pawn>(e).ok().map(|p| format!("{} {:?} hp {}", p.name, p.pos, p.hp)))
        .collect();
    let mut things: Vec<String> =
        w.ecs.query::<&Thing>().iter().map(|t| format!("{} {:?}", w.defs.thing(t.def).id, t.pos)).collect();
    things.sort();
    out.extend(things);
    out
}

fn play(dir: &std::path::Path, ticks: u64) -> Sim {
    let mut s = Sim::new(dir, 5).unwrap_or_else(|e| panic!("loads: {e}"));
    for _ in 0..ticks {
        s.step();
    }
    s
}

#[test]
fn a_mod_drawing_every_tick_changes_nothing_else() {
    let plain = test_mods("streams-plain", &["core"], &[]);
    let dicey = test_mods(
        "streams-dicey",
        &["core"],
        &[(
            "dice",
            &[(
                "scripts/dice.luau",
                "rim.every(1, function() rim.random(); rim.random_int(1, 6); rim.edge_cell() end)",
            )],
        )],
    );
    let (a, b) = (play(&plain, 3 * TICKS_PER_DAY), play(&dicey, 3 * TICKS_PER_DAY));
    assert!(!a.world.pawns.is_empty(), "there are pawns to compare");
    assert_eq!(outcome(&a), outcome(&b), "a mod's dice are its own");
    let _ = (fs::remove_dir_all(plain), fs::remove_dir_all(dicey));
}

/// A mod that brings people in doesn't move where plants spread: a new pawn's
/// name, skills and first think are drawn by its own id, not from `spawns`.
#[test]
fn a_mod_spawning_people_leaves_plant_spread_alone() {
    let plain = test_mods("streams-quiet", &["core"], &[]);
    let busy = test_mods(
        "streams-arrivals",
        &["core"],
        &[(
            "arrivals",
            &[(
                "scripts/arrivals.luau",
                r#"rim.every(200, function()
                    local x, y = rim.colony_center()
                    if x then rim.spawn_pawn("core:human", "player", x + 1, y) end
                end)"#,
            )],
        )],
    );
    let (a, b) = (play(&plain, TICKS_PER_DAY), play(&busy, TICKS_PER_DAY));
    assert!(b.world.pawns.len() > a.world.pawns.len(), "the mod brought people in");
    assert_eq!(natural(&a), natural(&b), "and plants spread just the same");
    let _ = (fs::remove_dir_all(plain), fs::remove_dir_all(busy));
}

/// Where every natural thing stands: what `spawns` decides after mapgen.
fn natural(s: &Sim) -> Vec<String> {
    let w = &s.world;
    let mut v: Vec<String> = w
        .ecs
        .query::<&Thing>()
        .iter()
        .filter(|t| w.defs.thing(t.def).natural)
        .map(|t| format!("{} {:?}", w.defs.thing(t.def).id, t.pos))
        .collect();
    v.sort();
    v
}

#[test]
fn extra_ai_draws_leave_spawns_alone() {
    let dir = test_mods("streams-ai", &["core"], &[]);
    let mut a = Sim::new(&dir, 9).unwrap();
    let mut b = Sim::new(&dir, 9).unwrap();
    for _ in 0..TICKS_PER_DAY / 2 {
        b.world.streams.stream(AI).next_u64();
        a.step();
        b.step();
    }
    // Plants spread from the spawns stream; the AI draws apart from it.
    assert_eq!(natural(&a), natural(&b), "the AI's draws don't move where plants spread");
    assert_ne!(
        a.world.state_hash(),
        b.world.state_hash(),
        "the extra draws are in the saved state, so the check isn't vacuous"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn streams_are_independent_and_restore_exactly() {
    let mut s = Streams::new(42);
    let spawns: Vec<u64> = (0..5).map(|_| s.stream(SPAWNS).next_u64()).collect();
    let mut t = Streams::new(42);
    for _ in 0..100 {
        t.stream(AI).next_u64();
    }
    let again: Vec<u64> = (0..5).map(|_| t.stream(SPAWNS).next_u64()).collect();
    assert_eq!(spawns, again, "drawing from ai doesn't move spawns");
    let mut u = Streams::restore(42, &t.states());
    assert_eq!(u.stream(SPAWNS).next_u64(), t.stream(SPAWNS).next_u64(), "a restored stream carries on");
    assert_eq!(u.stream("never-opened").next_u64(), Streams::new(42).stream("never-opened").next_u64());
    assert_eq!(s.draw(AI, 7, 100, 0).next_u64(), s.draw(AI, 7, 100, 0).next_u64(), "a draw is a pure function");
    assert_ne!(s.draw(AI, 7, 100, 0).next_u64(), s.draw(AI, 8, 100, 0).next_u64(), "of who draws");
}
