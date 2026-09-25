//! Work progress lives on the thing (DESIGN.md §6b): it survives the
//! worker leaving, a second worker and a save, and the map is touched when
//! a site starts or stops being worked, never on progress.

use rim_sim::defs::{DefId, Targets};
use rim_sim::hecs::Entity;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{Blueprint, Faction, Job, Pawn, Side, Thing, Work};
use rim_sim::{Command, IVec, Sim};
use std::path::Path;

fn mods() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

/// A game with the founder alone, so nobody else takes the job.
fn sim() -> (Sim, Entity) {
    let mut s = Sim::new(&mods(), 21).expect("mods load");
    s.step();
    let founder = s.world.colonists().next().expect("a founder");
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

fn job(s: &Sim, e: Entity) -> Job {
    s.world.ecs.get::<&Pawn>(e).unwrap().job.clone()
}

fn work(s: &Sim, e: Entity) -> Option<Work> {
    s.world.ecs.get::<&Work>(e).ok().map(|w| *w)
}

fn step_until(s: &mut Sim, cap: u32, done: impl Fn(&Sim) -> bool) -> bool {
    for _ in 0..cap {
        if done(s) {
            return true;
        }
        s.step();
    }
    done(s)
}

/// The nearest oak the founder can reach.
fn nearest_oak(s: &mut Sim, founder: Entity) -> (Entity, IVec) {
    let oak = thing(s, "tree_oak");
    let from = s.world.pawn_pos(founder).unwrap();
    s.world.map.ensure_regions();
    let mut best: Option<(u32, Entity, IVec)> = None;
    for (e, t) in s.world.ecs.query::<(Entity, &Thing)>().iter() {
        let d = t.pos.octile(from);
        if t.def == oak
            && best.is_none_or(|b| d < b.0)
            && s.world.map.can_reach(from, rim_sim::path::Goal::Touch(t.pos))
        {
            best = Some((d, e, t.pos));
        }
    }
    best.map(|b| (b.1, b.2)).expect("a reachable oak")
}

fn open_cell(s: &Sim) -> IVec {
    let c = s.world.colony_center().expect("a colony");
    (1..30)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))))
        .find(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none())
        .expect("an open cell")
}

/// The oak's harvest that fells it: the chop, whatever else it offers.
fn felling(s: &Sim) -> rim_sim::defs::HarvestDef {
    let oak = thing(s, "tree_oak");
    s.world.defs.thing(oak).harvest.iter().find(|h| h.destroy).expect("an oak can be felled").clone()
}

fn chop(s: &mut Sim, at: IVec) {
    let chop = felling(s).desig_r;
    s.push(Command::Designate { designation: chop, a: at, b: at });
}

/// Chop until the tree has some progress on it, then send the founder away.
fn half_chop(s: &mut Sim, founder: Entity) -> (Entity, IVec, u32) {
    let (tree, at) = nearest_oak(s, founder);
    chop(s, at);
    assert!(step_until(s, 4000, |s| work(s, tree).is_some_and(|w| w.done >= 40)), "the chop gets under way");
    let done = work(s, tree).unwrap().done;
    s.push(Command::Draft { pawn: founder, on: true });
    s.step();
    (tree, at, done)
}

#[test]
fn an_interrupted_chop_resumes_where_it_stopped() {
    let (mut s, founder) = sim();
    let (tree, _, done) = half_chop(&mut s, founder);
    for _ in 0..300 {
        s.step();
    }
    assert!(!matches!(job(&s, founder), Job::Harvest { .. }), "drafted, off the job");
    assert!(work(&s, tree).unwrap().done >= done, "the progress stayed on the tree");

    s.push(Command::Draft { pawn: founder, on: false });
    assert!(step_until(&mut s, 4000, |s| s.world.is_worksite(tree)), "back at it");
    assert!(work(&s, tree).unwrap().done > done, "carrying on from where it stopped, not from zero");
}

#[test]
fn a_second_pawn_carries_on_from_the_first() {
    let (mut s, founder) = sim();
    let (tree, _, _) = half_chop(&mut s, founder);
    let here = s.world.pawn_pos(founder).unwrap();
    let second = s.world.spawn_pawn(s.world.defs.creature_id("human").unwrap(), Faction::Player, here, None);
    s.step();
    assert!(step_until(&mut s, 6000, |s| matches!(job(s, second), Job::Harvest { target, .. } if target == tree)));
    // Progress only ever goes up: the second pawn never starts it over.
    let mut last = work(&s, tree).unwrap().done;
    while s.world.thing(tree).is_some() {
        s.step();
        if let Some(k) = work(&s, tree) {
            assert!(k.done >= last, "went from {last} back to {}", k.done);
            last = k.done;
        }
        assert!(s.world.tick < 20_000, "the tree comes down");
    }
}

#[test]
fn take_down_progress_survives_the_job_ending() {
    let (mut s, founder) = sim();
    let (wall, stone) = (thing(&s, "wall"), thing(&s, "stone"));
    let at = open_cell(&s);
    let w = s.world.spawn_fixture_of(wall, at, false, Some(stone)).expect("placed");
    rim_sim::ai::complete_building(&mut s.world, w);
    let decon =
        s.world.defs.designations.iter().position(|d| d.targets == Targets::Built).expect("deconstruct") as DefId;
    s.push(Command::Designate { designation: decon, a: at, b: at });
    assert!(step_until(&mut s, 4000, |s| work(s, w).is_some_and(|k| k.done >= 20)), "under way");
    let k = work(&s, w).unwrap();
    assert_eq!(k.designation, Some(decon));
    s.push(Command::Draft { pawn: founder, on: true });
    for _ in 0..200 {
        s.step();
    }
    assert!(!matches!(job(&s, founder), Job::Deconstruct { .. }));
    assert!(work(&s, w).unwrap().done >= k.done, "kept on the wall");
}

#[test]
fn a_plan_keeps_its_progress_in_work() {
    let (mut s, _) = sim();
    let (wall, stone) = (thing(&s, "wall"), thing(&s, "stone"));
    let at = open_cell(&s);
    let bp = s.world.spawn_fixture_of(wall, at, true, Some(stone)).expect("a plan");
    let k = work(&s, bp).expect("a plan starts with its work");
    assert_eq!((k.done, k.designation), (0, None));
    assert_eq!(k.total as f64, s.world.stat(bp, "work").unwrap().round(), "the material scales it");
    assert!(s.world.ecs.get::<&Blueprint>(bp).is_ok());
}

#[test]
fn work_survives_a_save() {
    let (mut s, founder) = sim();
    let (tree, _, _) = half_chop(&mut s, founder);
    let before = work(&s, tree).unwrap();
    let snap = Snapshot::capture(&s);
    let loaded = snap.restore(&mods(), &|_| true).expect("restores");
    assert_eq!(work(&loaded, tree), Some(before));
    assert_eq!(Snapshot::capture(&loaded).hash(), snap.hash(), "the same world");
}

#[test]
fn a_format_1_plan_keeps_its_progress() {
    #[derive(serde::Serialize)]
    struct OldBlueprint {
        cost: Vec<(DefId, u32)>,
        delivered: Vec<u32>,
        work: u32,
        work_left: u32,
    }
    let (mut s, _) = sim();
    let (wall, stone) = (thing(&s, "wall"), thing(&s, "stone"));
    let bp = s.world.spawn_fixture_of(wall, open_cell(&s), true, Some(stone)).expect("a plan");
    let total = work(&s, bp).unwrap().total;
    let mut snap = Snapshot::capture(&s);

    // Rewrite it the way format 1 wrote plans: progress on the Blueprint.
    snap.header.format = 1;
    snap.sections.remove("engine:work");
    let old =
        vec![(bp, OldBlueprint { cost: vec![(stone, 5)], delivered: vec![5], work: total, work_left: total - 30 })];
    snap.sections.insert("engine:blueprint".into(), rmp_serde::to_vec_named(&old).unwrap());

    let loaded = snap.restore(&mods(), &|_| true).expect("a format-1 save loads");
    let k = work(&loaded, bp).expect("its progress moved to Work");
    assert_eq!((k.done, k.total, k.designation), (30, total, None));
    assert_eq!(loaded.world.ecs.get::<&Blueprint>(bp).unwrap().delivered, vec![5]);
}

#[test]
fn a_format_1_chop_keeps_its_progress() {
    let (mut s, founder) = sim();
    let (tree, _) = nearest_oak(&mut s, founder);
    let hd = felling(&s);
    s.world.ecs.get::<&mut Pawn>(founder).unwrap().job =
        Job::Harvest { target: tree, forced: true, harvest: hd.key(), tool: None };
    let mut snap = Snapshot::capture(&s);

    // Format 1 counted a chop on the pawn's job, and plans on the Blueprint.
    snap.header.format = 1;
    snap.sections.remove("engine:work");
    let mut pawns: Vec<(Entity, serde_json::Value)> = rmp_serde::from_slice(&snap.sections["engine:pawn"]).unwrap();
    for (e, p) in &mut pawns {
        if *e == founder {
            p["job"]["Harvest"]["work"] = 50.into();
        }
    }
    snap.sections.insert("engine:pawn".into(), rmp_serde::to_vec_named(&pawns).unwrap());
    let mut plans: Vec<(Entity, serde_json::Value)> =
        rmp_serde::from_slice(&snap.sections["engine:blueprint"]).unwrap();
    for (_, b) in &mut plans {
        b["work"] = 10.into();
        b["work_left"] = 10.into();
    }
    snap.sections.insert("engine:blueprint".into(), rmp_serde::to_vec_named(&plans).unwrap());

    let loaded = snap.restore(&mods(), &|_| true).expect("a format-1 save loads");
    let k = work(&loaded, tree).expect("the job's progress moved to the tree");
    assert_eq!((k.done, k.total, k.designation), (50, hd.work, Some(hd.desig_r)));
}

#[test]
fn a_newer_format_is_refused() {
    let (s, _) = sim();
    let mut snap = Snapshot::capture(&s);
    snap.header.format = rim_sim::snapshot::FORMAT + 1;
    assert!(snap.restore(&mods(), &|_| true).is_err());
}

#[test]
fn the_map_is_touched_when_work_starts_and_stops_not_on_progress() {
    let (mut s, founder) = sim();
    let (tree, at) = nearest_oak(&mut s, founder);
    chop(&mut s, at);
    let chunk = s.world.map.chunk_of(at);
    assert!(step_until(&mut s, 4000, |s| s.world.is_worksite(tree)), "work starts");
    let started = s.world.map.things_rev(chunk);
    let done = work(&s, tree).unwrap().done;
    for _ in 0..40 {
        s.step();
    }
    assert!(work(&s, tree).unwrap().done > done, "progress was made");
    assert_eq!(s.world.map.things_rev(chunk), started, "and the chunk wasn't touched for it");

    s.push(Command::Draft { pawn: founder, on: true });
    assert!(step_until(&mut s, 400, |s| !s.world.is_worksite(tree)), "the site goes quiet");
    assert!(s.world.map.things_rev(chunk) > started, "which touches the chunk once more");
}

#[test]
fn stage_reads_work_and_lost_hp() {
    let (mut s, _) = sim();
    let (wall, stone) = (thing(&s, "wall"), thing(&s, "stone"));
    let at = open_cell(&s);
    let w = s.world.spawn_fixture_of(wall, at, false, Some(stone)).expect("placed");
    assert_eq!(s.world.stage(w), 0);
    let max = s.world.stat(w, "hp").unwrap().round() as i32;
    s.world.ecs.get::<&mut Thing>(w).unwrap().hp = max / 2;
    assert_eq!(s.world.stage(w), 4, "half its hp gone");
    s.world.ecs.insert_one(w, Work { done: 7, total: 8, designation: None, side: Side::East }).unwrap();
    assert_eq!(s.world.stage(w), 7, "the larger of the two");
}

#[test]
fn side_is_where_the_worker_stands() {
    let at = IVec::new(5, 5);
    assert_eq!(Side::of(at, IVec::new(4, 5)), Side::West);
    assert_eq!(Side::of(at, IVec::new(6, 5)), Side::East);
    assert_eq!(Side::of(at, IVec::new(5, 4)), Side::North);
    assert_eq!(Side::of(at, IVec::new(5, 6)), Side::South);
    assert_eq!(Side::of(at, IVec::new(6, 6)), Side::East, "a diagonal tie counts east or west");
}
