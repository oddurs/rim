//! The stone age's first tier (mods/primitive): gather with bare hands, and
//! see the first night through with branches.

mod common;

use rim_sim::hecs::Entity;
use rim_sim::world::{Blueprint, Thing};
use rim_sim::{Command, IVec, Sim};

/// One colonist, so nobody else takes the work.
fn alone(seed: u64) -> (Sim, Entity) {
    let mut s = Sim::new(&common::mods(), seed).unwrap_or_else(|e| panic!("mods load: {e}"));
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

fn count(s: &Sim, id: &str) -> u32 {
    let d = s.world.defs.thing_id(id).unwrap_or_else(|| panic!("thing {id}"));
    s.world.ecs.query::<&Thing>().without::<&Blueprint>().iter().filter(|t| t.def == d).map(|t| t.count).sum()
}

/// The nearest reachable thing of a def.
fn nearest(s: &Sim, id: &str) -> Option<(Entity, IVec)> {
    let d = s.world.defs.thing_id(id).unwrap();
    let c = s.world.colony_center().unwrap();
    s.world
        .ecs
        .query::<(Entity, &Thing)>()
        .iter()
        .filter(|(_, t)| t.def == d && s.world.map.can_reach(c, rim_sim::path::Goal::Touch(t.pos)))
        .map(|(e, t)| (e, t.pos))
        .min_by_key(|(e, p)| (p.octile(c), e.id()))
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

#[test]
fn the_wild_offers_branches_fibre_stones_and_flint() {
    // Over a few seeds, every wild thing turns up somewhere.
    for id in ["primitive:deadfall", "primitive:tall_grass", "primitive:loose_stones", "primitive:flint_nodule"] {
        let found = (1..=5).any(|seed| {
            let s = Sim::new(&common::mods(), seed).unwrap();
            count(&s, id) > 0
        });
        assert!(found, "{id} spawns on some map");
    }
}

#[test]
fn branches_come_off_an_oak_by_hand_and_the_oak_stands() {
    let (mut s, _) = alone(3);
    let (oak, at) = nearest(&s, "tree_oak").expect("an oak");
    let gather = s.world.defs.lookup("designation", "core:gather").unwrap();
    s.push(Command::Designate { designation: gather, a: at, b: at });
    assert!(run_until(&mut s, 4_000, |s| count(s, "primitive:branches") >= 3), "branches gathered");
    assert!(s.world.thing(oak).is_some(), "the oak still stands");
}

#[test]
fn deadfall_and_tall_grass_are_gathered_whole() {
    let (mut s, _) = alone(3);
    let gather = s.world.defs.lookup("designation", "core:gather").unwrap();
    for (wild, item, n) in
        [("primitive:deadfall", "primitive:branches", 4), ("primitive:tall_grass", "primitive:fibre", 3)]
    {
        let (e, at) = nearest(&s, wild).unwrap_or_else(|| panic!("a {wild} in reach"));
        let before = count(&s, item);
        s.push(Command::Designate { designation: gather, a: at, b: at });
        assert!(run_until(&mut s, 4_000, |s| s.world.thing(e).is_none()), "{wild} gathered");
        assert_eq!(count(&s, item), before + n, "{wild} gives {n} {item}");
    }
}

/// Gather, haul, build: the campfire's branches come off the land, with
/// no axe and nothing placed by hand.
#[test]
fn a_campfire_of_gathered_branches_needs_no_axe() {
    let (mut s, founder) = alone(3);
    let campfire = s.world.defs.thing_id("campfire").unwrap();
    let branches = s.world.defs.thing_id("primitive:branches").unwrap();
    let cost = &s.world.defs.thing(campfire).build.as_ref().unwrap().cost_r;
    assert_eq!(cost, &vec![(branches, 10)], "the plugin's patch");

    let home = s.world.pawn_pos(founder).unwrap();
    let gather = s.world.defs.lookup("designation", "core:gather").unwrap();
    s.push(Command::Designate { designation: gather, a: home.offset(-20, -20), b: home.offset(20, 20) });
    let at = (1..6)
        .map(|d| home.offset(d, 0))
        .find(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none())
        .expect("a free cell");
    s.push(Command::Build { thing: campfire, stuff: None, a: at, b: at });
    assert!(
        run_until(&mut s, 20_000, |s| s.world.map.fixture_at(at).is_some_and(|f| s
            .world
            .ecs
            .get::<&Blueprint>(f)
            .is_err())),
        "the campfire is built within a day"
    );
}

/// With the stone age on, a click on an oak gathers from it, and a second
/// click while its branches grow back doesn't fell it instead.
#[test]
fn a_click_gathers_from_an_oak_and_never_fells_it() {
    let (mut s, founder) = alone(3);
    let (oak, at) = nearest(&s, "tree_oak").expect("an oak");
    let order = |s: &Sim| rim_sim::order::resolve(&s.world, founder, at, None).map(|o| o.label);
    assert_eq!(order(&s).as_deref(), Some("Gather oak tree"));
    let gather = s.world.defs.lookup("designation", "core:gather").unwrap();
    s.push(Command::Designate { designation: gather, a: at, b: at });
    assert!(run_until(&mut s, 4_000, |s| !s.world.harvest_ready(oak, Some(gather))), "gathered");
    assert_ne!(order(&s).as_deref(), Some("Chop oak tree"));
    assert!(s.world.thing(oak).is_some());
}

#[test]
fn branches_build_walls() {
    let s = Sim::new(&common::mods(), 1).unwrap();
    let branches = s.world.defs.thing_id("primitive:branches").unwrap();
    let wall = s.world.defs.thing_id("wall").unwrap();
    assert!(
        s.world.defs.is_material_for(branches, "structural")
            && s.world
                .defs
                .thing(wall)
                .build
                .as_ref()
                .unwrap()
                .stuff
                .as_ref()
                .is_some_and(|c| c.category == "structural"),
        "branches build walls"
    );
}

/// With the plugin removed, core plays exactly as it did (DESIGN.md §5).
#[test]
fn core_alone_is_untouched() {
    let s = Sim::build(&common::mods(), 3, &|m| m == "core", 250).unwrap();
    let d = &s.world.defs;
    assert_eq!(d.thing(d.thing_id("tree_oak").unwrap()).harvest.len(), 1);
    let wood = d.thing_id("wood").unwrap();
    assert_eq!(d.thing(d.thing_id("campfire").unwrap()).build.as_ref().unwrap().cost_r, vec![(wood, 15)]);
}
