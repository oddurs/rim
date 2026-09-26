//! Urgency within a priority level (DESIGN.md §4d): a colony with no
//! shelter raises one before it gathers what's beside it; otherwise the
//! nearest job still wins.

mod common;

use rim_sim::hecs::Entity;
use rim_sim::world::{Blueprint, Job, Pawn, Thing};
use rim_sim::{Command, IVec, Sim};

fn core() -> Sim {
    Sim::with_mods(&common::mods(), 2, &|m| m == "core").unwrap()
}

/// The first work the founder takes up over `ticks`.
fn first_job(s: &mut Sim, pawn: Entity, ticks: u32) -> Option<Job> {
    for _ in 0..ticks {
        s.step();
        let job = s.world.ecs.get::<&Pawn>(pawn).unwrap().job.clone();
        if matches!(job, Job::Harvest { .. } | Job::Construct { .. } | Job::Deliver { .. }) {
            return Some(job);
        }
    }
    None
}

/// Mark the oak nearest `at` to chop, and give the founder wood beside them.
fn setup(s: &mut Sim) -> (Entity, IVec) {
    let pawn = s.world.colonists().next().unwrap();
    let at = s.world.pawn_pos(pawn).unwrap();
    let oak = s.world.defs.thing_id("tree_oak").unwrap();
    let (_, tree) = s
        .world
        .ecs
        .query::<(Entity, &Thing)>()
        .iter()
        .filter(|(_, t)| t.def == oak)
        .map(|(e, t)| (e, t.pos))
        .min_by_key(|(e, p)| (p.octile(at), e.id()))
        .unwrap();
    let chop = s.world.defs.lookup("designation", "chop").unwrap();
    s.push(Command::Designate { designation: chop, a: tree, b: tree });
    let wood = s.world.defs.thing_id("wood").unwrap();
    s.world.place_item(wood, at.offset(1, 0), 60);
    (pawn, at)
}

/// Open ground about `r` cells from `at`: nothing to clear before a plan.
fn open_at(s: &Sim, at: IVec, r: i32) -> IVec {
    (r..r + 20)
        .flat_map(|k| [at.offset(k, 0), at.offset(-k, 0), at.offset(0, k), at.offset(0, -k)])
        .find(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none())
        .expect("open ground")
}

/// Whether a job works on a blueprint of `def`.
fn builds(s: &Sim, job: &Job, def: &str) -> bool {
    let d = s.world.defs.thing_id(def).unwrap();
    let bp = match job {
        Job::Construct { bp } | Job::Deliver { bp, .. } => *bp,
        _ => return false,
    };
    s.world.ecs.get::<&Blueprint>(bp).is_ok() && s.world.thing(bp).is_some_and(|t| t.def == d)
}

#[test]
fn with_no_shelter_a_far_wall_beats_a_near_tree() {
    let mut s = core();
    let (pawn, at) = setup(&mut s);
    let wall = s.world.defs.thing_id("wall").unwrap();
    let wood = s.world.defs.thing_id("wood");
    // Far off: every tree near the colony is nearer than this wall.
    let far = open_at(&s, at, 18);
    s.push(Command::Build { thing: wall, stuff: wood, a: far, b: far });
    assert!(!s.world.has_shelter());
    let job = first_job(&mut s, pawn, 600).expect("some work");
    assert!(builds(&s, &job, "wall"), "shelter first: {job:?}");
}

#[test]
fn a_far_table_waits_for_the_near_tree() {
    // A table is no shelter: it's calm work, and the nearer job wins.
    let mut s = core();
    let (pawn, at) = setup(&mut s);
    let table = s.world.defs.thing_id("table").unwrap();
    let wood = s.world.defs.thing_id("wood");
    let far = open_at(&s, at, 18);
    s.push(Command::Build { thing: table, stuff: wood, a: far, b: far });
    let job = first_job(&mut s, pawn, 600).expect("some work");
    assert!(matches!(job, Job::Harvest { .. }), "the near tree first: {job:?}");
}

#[test]
fn between_urgent_plans_the_nearest_wins() {
    let mut s = core();
    let (pawn, at) = setup(&mut s);
    let wall = s.world.defs.thing_id("wall").unwrap();
    let wood = s.world.defs.thing_id("wood");
    let (far, near) = (open_at(&s, at, 18), open_at(&s, at, 3));
    s.push(Command::Build { thing: wall, stuff: wood, a: far, b: far });
    s.push(Command::Build { thing: wall, stuff: wood, a: near, b: near });
    let job = first_job(&mut s, pawn, 600).expect("some work");
    let bp = match job {
        Job::Construct { bp } | Job::Deliver { bp, .. } => bp,
        other => panic!("{other:?}"),
    };
    assert_eq!(s.world.thing(bp).unwrap().pos, near, "the near wall, not the far one");
}

#[test]
fn a_priority_level_still_beats_urgency() {
    let mut s = core();
    let (pawn, at) = setup(&mut s);
    let wall = s.world.defs.thing_id("wall").unwrap();
    let wood = s.world.defs.thing_id("wood");
    let far = open_at(&s, at, 18);
    s.push(Command::Build { thing: wall, stuff: wood, a: far, b: far });
    let chop = s.world.defs.lookup("work_type", "core:chop").unwrap();
    s.push(Command::SetPriority { pawn, work: chop, level: 1 });
    let job = first_job(&mut s, pawn, 600).expect("some work");
    assert!(matches!(job, Job::Harvest { .. }), "chop at 1 before build at 3, urgent or not: {job:?}");
}

/// Choosing work reads the rooms as the step started with them: rebuilding
/// them mid-tick would renumber rooms under the fields' room values, and a
/// warm hut would read another room's temperature.
#[test]
fn choosing_work_never_rebuilds_rooms_mid_tick() {
    let mut s = core();
    let (pawn, at) = setup(&mut s);
    let wall = s.world.defs.thing_id("wall").unwrap();
    let wood = s.world.defs.thing_id("wood");
    let far = open_at(&s, at, 18);
    s.push(Command::Build { thing: wall, stuff: wood, a: far, b: far });
    s.step();
    // A wall goes up mid-tick; then the colonist thinks.
    let other = open_at(&s, at, 8);
    s.world.spawn_fixture_of(wall, other, false, wood);
    {
        let mut p = s.world.ecs.get::<&mut Pawn>(pawn).unwrap();
        p.job = Job::Idle;
        p.next_think = 0;
    }
    let rebuilds = s.world.map.room_rebuilds;
    rim_sim::ai::tick_pawns(&mut s.world);
    assert_eq!(s.world.map.room_rebuilds, rebuilds, "rooms rebuild at the start of a step, not while pawns think");
}
