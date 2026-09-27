//! Core's planner (DESIGN.md §4d): Auto fills the gaps the rest of the
//! colony leaves, then sets everything else by skill.

mod common;

use rim_sim::world::{Faction, Job, Pawn};
use rim_sim::{Command, Sim, TICKS_PER_DAY};

const HOUR: u64 = TICKS_PER_DAY / 24;

fn core(seed: u64) -> Sim {
    Sim::with_mods(&common::mods(), seed, &|m| m == "core").unwrap()
}

fn planned(s: &Sim, e: rim_sim::hecs::Entity, work: &str) -> Option<(u8, String)> {
    let w = s.world.defs.lookup("work_type", work).unwrap();
    let p = s.world.ecs.get::<&Pawn>(e).unwrap();
    p.planned(w).map(|x| (x.level, x.reason.clone()))
}

fn hours(s: &mut Sim, n: u64) {
    for _ in 0..n * HOUR {
        s.step();
    }
}

/// A colony of `n`, everyone on Auto.
fn colony(seed: u64, n: usize) -> Sim {
    let mut s = core(seed);
    let human = s.world.defs.creature_id("human").unwrap();
    let c = s.world.colony_center().unwrap();
    for i in 1..n {
        let at = c.offset(i as i32 % 3, i as i32 / 3);
        s.world.spawn_pawn(human, Faction::Player, if s.world.map.passable(at) { at } else { c }, None);
    }
    s
}

#[test]
fn auto_never_says_never_and_never_plans_a_pin() {
    let mut s = colony(3, 4);
    let colonists: Vec<_> = s.world.colonists().collect();
    let haul = s.world.defs.lookup("work_type", "core:haul").unwrap();
    s.push(Command::SetPriority { pawn: colonists[0], work: haul, level: 4 });
    s.world.messages.clear();
    hours(&mut s, 6);
    for &e in &colonists {
        let p = s.world.ecs.get::<&Pawn>(e).unwrap();
        assert!(!p.plan.is_empty(), "{} has a plan", p.name);
        assert!(p.plan.iter().all(|x| x.level > 0), "{}: {:?}", p.name, p.plan);
    }
    assert_eq!(planned(&s, colonists[0], "core:haul"), None, "the pin is the player's");
    let said: Vec<&str> = s.world.messages.iter().map(|m| m.text.as_str()).collect();
    assert!(!said.iter().any(|m| m.contains("planner core:auto")), "the planner broke no rule: {said:?}");
}

/// Blueprints that a Builder already covers need no Auto colonist at First.
#[test]
fn a_gap_the_colony_covers_is_left_alone() {
    let mut s = colony(4, 2);
    let (builder_c, auto_c) = {
        let v: Vec<_> = s.world.colonists().collect();
        (v[0], v[1])
    };
    let builder = s.world.work_roles.iter().position(|r| r.label == "Builder").unwrap() as u16;
    s.push(Command::AssignWorkRole { pawn: builder_c, role: builder });
    let defs = s.world.defs.clone();
    let c = s.world.colony_center().unwrap();
    let (wall, wood) = (defs.thing_id("wall").unwrap(), defs.thing_id("wood").unwrap());
    s.world.place_item(wood, c.offset(1, 1), 40);
    // Four blueprints: one person's worth at core's per_person = 4.
    s.push(Command::Build { thing: wall, stuff: Some(wood), a: c.offset(-6, 5), b: c.offset(-3, 5), facing: 0 });
    hours(&mut s, 1);
    let (level, reason) = planned(&s, auto_c, "core:build").expect("a plan for build");
    assert_ne!(level, 1, "the Builder covers it: {reason}");
    assert!(reason.starts_with("covered"), "{reason}");
}

/// A castaway on Auto, with a shelter planned and berries marked, builds
/// and forages on the first day with nobody touching a priority.
#[test]
fn a_castaway_on_auto_builds_and_forages() {
    let mut s = core(1);
    let pawn = s.world.colonists().next().unwrap();
    let defs = s.world.defs.clone();
    let c = s.world.pawn_pos(pawn).unwrap();
    let (wall, wood) = (defs.thing_id("wall").unwrap(), defs.thing_id("wood").unwrap());
    s.world.place_item(wood, c.offset(1, 1), 40);
    s.push(Command::Build { thing: wall, stuff: Some(wood), a: c.offset(-4, 3), b: c.offset(-1, 3), facing: 0 });
    let harvest = defs.lookup("designation", "harvest").unwrap();
    s.push(Command::Designate { designation: harvest, a: c.offset(-20, -20), b: c.offset(20, 20) });
    let (mut built, mut foraged) = (false, false);
    for _ in 0..TICKS_PER_DAY {
        s.step();
        match &s.world.ecs.get::<&Pawn>(pawn).unwrap().job {
            Job::Construct { .. } | Job::Deliver { .. } => built = true,
            Job::Harvest { .. } => foraged = true,
            _ => {}
        }
        if built && foraged {
            break;
        }
    }
    assert!(built && foraged, "built {built}, foraged {foraged}");
}

/// Six colonists and a pile of loose items with a stockpile to take them:
/// Auto puts someone on hauling within two hours.
#[test]
fn a_pile_of_loose_items_gets_a_hauler() {
    let mut s = colony(6, 6);
    let defs = s.world.defs.clone();
    let c = s.world.colony_center().unwrap();
    let wood = defs.thing_id("wood").unwrap();
    for i in 0..24 {
        let at = c.offset(-8 + (i % 6) * 2, 6 + (i / 6) * 2);
        if s.world.map.passable(at) {
            s.world.place_item(wood, at, 5);
        }
    }
    s.push(Command::Stockpile { a: c.offset(6, -6), b: c.offset(10, -2), zone: None });
    hours(&mut s, 2);
    let haulers: Vec<String> = s
        .world
        .colonists()
        .filter_map(|e| planned(&s, e, "core:haul").filter(|(l, _)| *l <= 2).map(|(_, r)| r))
        .collect();
    assert!(!haulers.is_empty(), "nobody on Haul");
    assert!(haulers.iter().any(|r| r.starts_with("fills Haul")), "{haulers:?}");
}

/// The same colony plans the same way, run after run.
#[test]
fn plans_are_deterministic() {
    let run = || {
        let mut s = colony(8, 5);
        hours(&mut s, 5);
        s.world.state_hash()
    };
    assert_eq!(run(), run());
}

/// What one run of core's planner costs at thirty colonists, for DESIGN.md
/// §4d: `cargo test --release -p rim_sim --test auto_planner -- --ignored
/// --nocapture`. Timed, so not asserted.
#[test]
#[ignore]
fn planner_cost_at_thirty_colonists() {
    let mut s = Sim::new(&common::mods(), 9).unwrap();
    let human = s.world.defs.creature_id("human").unwrap();
    let c = s.world.colony_center().unwrap();
    for i in 1..30 {
        let at = c.offset(i % 6, i / 6);
        s.world.spawn_pawn(human, Faction::Player, if s.world.map.passable(at) { at } else { c }, None);
    }
    hours(&mut s, 1);
    s.profile.reset_totals();
    hours(&mut s, 24);
    let (_, total, calls) =
        s.profile.totals.iter().find(|t| t.0 == "planner:core:auto").cloned().expect("the planner ran");
    println!(
        "core:auto at {} colonists × {} work types: {:.1} µs a run over {calls} runs",
        s.world.colonists().count(),
        s.world.defs.work_types.len(),
        total / calls as f64
    );
}
