//! Levels meet at portals (DESIGN.md §6d): stairs and ladders dug down from
//! the level above, a pit that leaves air, reach and paths that go through
//! them, and all of it through a save.

mod common;

use rim_sim::command::{Blocker, Place};
use rim_sim::map::Map;
use rim_sim::path::Goal;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{Blueprint, Faction, Thing};
use rim_sim::{Command, IVec, Sim};

/// Core alone: nothing waits on a tool.
fn core() -> Sim {
    Sim::build(&common::mods(), 3, &|m| m == "core", 64).expect("core loads")
}

fn run_until(s: &mut Sim, ticks: u32, done: impl Fn(&Sim) -> bool) -> bool {
    for _ in 0..ticks {
        s.step();
        if done(s) {
            return true;
        }
    }
    false
}

fn thing(s: &Sim, id: &str) -> rim_sim::defs::DefId {
    s.world.defs.thing_id(id).unwrap_or_else(|| panic!("thing {id}"))
}

/// Open ground near the colony a dig can start from.
fn dig_spot(s: &Sim) -> IVec {
    let c = s.world.colony_center().unwrap();
    (2..20)
        .flat_map(|r| (-r..=r).flat_map(move |d| [c.offset(r, d), c.offset(-r, d), c.offset(d, r), c.offset(d, -r)]))
        .find(|&p| s.world.can_dig(p) && s.world.map.can_reach(c, Goal::Touch(p)))
        .expect("somewhere to dig")
}

/// Build `what` at `p` and wait for it to stand.
fn build(s: &mut Sim, what: &str, p: IVec) {
    let def = thing(s, what);
    s.push(Command::Build { thing: def, stuff: None, a: p, b: p, facing: 0 });
    s.step();
    assert!(s.world.map.fixture_at(p).is_some(), "{what} planned at {p:?}");
    let done = |s: &Sim| s.world.map.fixture_at(p).is_none_or(|f| s.world.ecs.get::<&Blueprint>(f).is_err());
    assert!(run_until(s, 40_000, done), "{what} at {p:?} is built");
}

/// Mine the rock at `p` and wait for it to go.
fn mine(s: &mut Sim, p: IVec) {
    let mine = s.world.defs.lookup("designation", "core:mine").unwrap();
    s.push(Command::Designate { designation: mine, a: p, b: p });
    assert!(run_until(s, 40_000, |s| s.world.solid_at(p).is_none()), "{p:?} mined");
}

/// Stairs at `top`, then the rock beside their foot: a way down and room
/// to stand. Returns the open cell beside the foot.
fn stairwell(s: &mut Sim, top: IVec) -> IVec {
    build(s, "stairs", top);
    let foot = IVec::at(top.x, top.y, top.z - 1);
    assert_eq!(s.world.map.portals().iter().filter(|p| p.top == top).count(), 1, "a portal at {top:?}");
    let beside = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .map(|(dx, dy)| foot.offset(dx, dy))
        .into_iter()
        .find(|&q| s.world.solid_at(q).is_some_and(|r| r.thing_r.is_some()))
        .expect("workable rock beside the foot");
    mine(s, beside);
    beside
}

#[test]
fn core_alone_digs_to_the_bottom_and_brings_stone_up() {
    let mut s = core();
    assert_eq!(s.world.map.levels(), -3..=0);
    let mut at = dig_spot(&s);
    // Three stairwells: the surface to −1, −1 to −2, −2 to −3.
    for _ in 0..3 {
        at = stairwell(&mut s, at);
    }
    assert_eq!(at.z, -3);
    assert!(!s.world.can_dig(at), "the deepest level has nowhere to dig to");
    let c = s.world.colony_center().unwrap();
    s.world.map.ensure_regions();
    assert!(s.world.map.can_reach(c, Goal::Cell(at)), "the bottom is reached from the surface");

    // A stockpile up top: stone mined three levels down is carried up to it.
    let stone = thing(&s, "stone");
    let spot = (3..30)
        .flat_map(|r| [c.offset(r, 0), c.offset(-r, 0), c.offset(0, r), c.offset(0, -r)])
        .find(|&o| (0..3).all(|y| (0..3).all(|x| s.world.map.passable(o.offset(x, y)))))
        .expect("room for a stockpile");
    s.push(Command::Stockpile { a: spot, b: spot.offset(2, 2), zone: None });
    let stored_up_top = |s: &Sim| {
        s.world
            .ecs
            .query::<&Thing>()
            .iter()
            .any(|t| t.def == stone && t.pos.z == 0 && s.world.zones.at(&s.world.map, t.pos).is_some())
    };
    assert!(run_until(&mut s, 60_000, stored_up_top), "stone from below is carried up the stairs");
}

const WATCH: &str = r#"
rim.on("level_opened", function(e)
    local n = rim.get_data("opened") or 0
    rim.set_data("opened", n + 1)
    rim.set_data("deepest", e.z)
end)
"#;

#[test]
fn the_first_dig_into_a_level_is_news_once() {
    let dir = common::test_mods("portals-opened", &["core"], &[("watch", &[("scripts/watch.luau", WATCH)])]);
    let mut s = Sim::build(&dir, 3, &|_| true, 64).unwrap();
    let top = dig_spot(&s);
    let beside = stairwell(&mut s, top);
    // A second way into the same level says nothing new.
    let second = IVec::at(beside.x, beside.y, 0);
    if s.world.can_dig(second) {
        build(&mut s, "ladder", second);
    }
    for _ in 0..5 {
        s.step();
    }
    let data = format!("{:?} {:?}", s.world.data.get("watch:opened"), s.world.data.get("watch:deepest"));
    assert!(data.contains("Int(1)") && data.contains("Int(-1)"), "{data}");
    let _ = std::fs::remove_dir_all(dir);
}

/// Open floor on two levels, joined by one portal.
fn two_floors(owner: Option<Faction>) -> Map {
    let mut m = Map::with_levels(12, 12, 1, 0);
    for y in 0..12 {
        for x in 0..12 {
            m.set_terrain(IVec::at(x, y, -1), 0, 100);
        }
    }
    m.add_portal(IVec::new(3, 3), 250, owner);
    m.ensure_regions();
    m
}

#[test]
fn reach_and_paths_go_through_a_portal() {
    let m = two_floors(None);
    let (top, far) = (IVec::new(9, 9), IVec::at(10, 2, -1));
    assert!(m.can_reach(top, Goal::Cell(far)));
    let mut pf = rim_sim::path::Pathfinder::default();
    let path = pf.find(&m, top, Goal::Cell(far), 10_000, Faction::Player).expect("a path");
    assert!(path.contains(&IVec::new(3, 3)) && path.contains(&IVec::at(3, 3, -1)), "down the stairs: {path:?}");
    assert_eq!(path.first(), Some(&far));

    // The colony's own stairs are a wall to a raider.
    let m = two_floors(Some(Faction::Player));
    assert!(m.can_reach_for(top, Goal::Cell(far), Faction::Player));
    assert!(!m.can_reach_for(top, Goal::Cell(far), Faction::Hostile));
    assert!(pf.find(&m, top, Goal::Cell(far), 10_000, Faction::Hostile).is_none());
}

#[test]
fn stairs_and_what_they_join_survive_a_save() {
    let mut s = core();
    let top = dig_spot(&s);
    let beside = stairwell(&mut s, top);
    let portals = s.world.map.portals().to_vec();
    let back = Snapshot::capture(&s).restore(&common::mods(), &|m| m == "core").unwrap();
    assert_eq!(back.world.map.portals(), portals.as_slice());
    let mut back = back;
    back.world.map.ensure_regions();
    let c = back.world.colony_center().unwrap();
    assert!(back.world.map.can_reach(c, Goal::Cell(beside)), "still reached after a load");
    assert!(Snapshot::capture(&back) == Snapshot::capture(&s), "save, load, save is the same");
}

#[test]
fn a_pit_is_air_and_the_rock_under_it_is_gone() {
    let mut s = core();
    let p = dig_spot(&s);
    let below = IVec::at(p.x, p.y, -1);
    build(&mut s, "pit", p);
    let i = s.world.map.idx(p);
    assert!(s.world.map.is_air(i), "the pit is air");
    assert!(!s.world.map.passable(p), "and nothing walks it");
    assert!(s.world.map.fixture_at(p).is_none(), "the dig site is gone");
    assert!(s.world.map.air_cells(0).contains(&(i as u32)));
    assert!(s.world.solid_at(below).is_none() && s.world.map.passable(below), "the cell below is dug out");
    assert!(!s.world.can_dig(p), "a pit can't be dug again");
}

#[test]
fn stairs_on_the_lowest_level_are_refused_and_the_preview_says_why() {
    let mut s = core();
    let c = s.world.colony_center().unwrap();
    let bottom = IVec::at(c.x, c.y, -3);
    let stairs = thing(&s, "stairs");
    let preview = rim_sim::command::build_preview(&s.world, stairs, None, bottom, bottom, 0);
    assert_eq!(preview, vec![(bottom, Place::Blocked(Blocker::NoDig))]);
    s.push(Command::Build { thing: stairs, stuff: None, a: bottom, b: bottom, facing: 0 });
    s.step();
    assert!(s.world.map.fixture_at(bottom).and_then(|f| s.world.thing(f)).is_none_or(|t| t.def != stairs));
}
