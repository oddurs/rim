//! The stone age (mods/primitive): gather with bare hands, see the first
//! night through with branches, and knap the tools that fell and quarry.

mod common;

use rim_sim::data::{Data, Key};
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
    for id in [
        "primitive:deadfall",
        "primitive:tall_grass",
        "primitive:loose_stones",
        "primitive:flint_nodule",
        "primitive:clay_bank",
    ] {
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
        [("primitive:deadfall", "primitive:branches", 6), ("primitive:tall_grass", "primitive:fibre", 3)]
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

/// Core's heavy work waits for a tool, and the thing says which.
#[test]
fn felling_and_quarrying_wait_for_tools_and_say_so() {
    let (mut s, founder) = alone(3);
    let (oak, at) = nearest(&s, "core:tree_oak").expect("an oak");
    let (rock, rat) = nearest(&s, "core:granite").expect("granite");
    for (des, p) in [("core:chop", at), ("core:mine", rat)] {
        let designation = s.world.defs.lookup("designation", des).unwrap();
        s.push(Command::Designate { designation, a: p, b: p });
    }
    s.step();
    assert_eq!(rim_sim::ai::work_blocked(&s.world, oak).as_deref(), Some("Needs a chopping tool."));
    assert_eq!(rim_sim::ai::work_blocked(&s.world, rock).as_deref(), Some("Needs a pounding tool."));
    for _ in 0..3_000 {
        s.step();
    }
    assert!(s.world.thing(oak).is_some() && s.world.thing(rock).is_some(), "nothing bare-handed");

    // A hammerstone breaks stone, slowly; still no axe for the oak.
    let home = s.world.pawn_pos(founder).unwrap();
    let hammer = s.world.defs.thing_id("primitive:hammerstone").unwrap();
    s.world.place_item(hammer, home, 1);
    assert!(run_until(&mut s, 12_000, |s| s.world.thing(rock).is_none()), "quarried with a hammerstone");
    assert!(s.world.thing(oak).is_some());
    assert_eq!(rim_sim::ai::work_blocked(&s.world, oak).as_deref(), Some("Needs a chopping tool."));
}

/// A tool is made of what it was knapped from: one def, its quality from
/// the material.
#[test]
fn a_hand_axe_is_made_of_its_flint() {
    let (mut s, founder) = alone(3);
    let home = s.world.pawn_pos(founder).unwrap();
    let def = |s: &Sim, id: &str| s.world.defs.thing_id(id).unwrap();
    let at = (2..10)
        .flat_map(|d| [home.offset(d, 0), home.offset(-d, 0), home.offset(0, d), home.offset(0, -d)])
        .find(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none())
        .expect("room for a spot");
    let spot = s.world.spawn_fixture_of(def(&s, "crafting:spot"), at, false, None).unwrap();
    rim_sim::ai::complete_building(&mut s.world, spot);
    s.world.place_item(def(&s, "primitive:flint"), home, 2);
    s.world.place_item(def(&s, "primitive:hammerstone"), home, 1);
    let data = [
        (Key::Str("site".into()), Data::Int(spot.to_bits().get() as i64)),
        (Key::Str("recipe".into()), Data::Str("primitive:hand_axe".into())),
    ];
    s.push(Command::ModEvent { name: "crafting:add_bill".into(), data: Some(Data::Table(data.into_iter().collect())) });
    let axe = def(&s, "primitive:hand_axe");
    let made = |s: &Sim| s.world.ecs.query::<&Thing>().iter().any(|t| t.def == axe);
    assert!(run_until(&mut s, 12_000, made), "a hand axe is knapped");
    let (e, hp) =
        s.world.ecs.query::<(Entity, &Thing)>().iter().find(|(_, t)| t.def == axe).map(|(e, t)| (e, t.hp)).unwrap();
    assert_eq!(s.world.made_of(e), Some(def(&s, "primitive:flint")));
    assert_eq!(hp, 60, "flint's hp factor is 1");
    assert!((s.world.tool_speed(e) - 0.6).abs() < 1e-9, "the axe's 0.6 at flint's 1.0");
}

/// A clay bank is dug with a digging stick, not bare hands, and isn't used
/// up: it's dug out for a few days.
#[test]
fn clay_is_dug_with_a_digging_stick() {
    let (mut s, founder) = (1..=10)
        .map(alone)
        .find(|(s, _)| nearest(s, "primitive:clay_bank").is_some())
        .expect("a reachable clay bank on some seed");
    let (bank, at) = nearest(&s, "primitive:clay_bank").unwrap();
    let gather = s.world.defs.lookup("designation", "core:gather").unwrap();
    s.push(Command::Designate { designation: gather, a: at, b: at });
    s.step();
    assert_eq!(rim_sim::ai::work_blocked(&s.world, bank).as_deref(), Some("Needs a digging tool."));
    for _ in 0..3_000 {
        s.step();
    }
    assert_eq!(count(&s, "primitive:clay"), 0, "not with bare hands");
    let home = s.world.pawn_pos(founder).unwrap();
    s.world.place_item(s.world.defs.thing_id("primitive:digging_stick").unwrap(), home, 1);
    assert!(run_until(&mut s, 12_000, |s| count(s, "primitive:clay") >= 4), "dug with a stick");
    assert!(s.world.thing(bank).is_some(), "and the bank is still there");
}

/// Cob (clay walls) keeps the warmth in better than branches do: a wall's
/// leak is divided by its material's insulation.
#[test]
fn cob_insulates_better_than_branches() {
    let (mut s, founder) = alone(3);
    let home = s.world.pawn_pos(founder).unwrap();
    let def = |s: &Sim, id: &str| s.world.defs.thing_id(id).unwrap();
    let wall = def(&s, "core:wall");
    let spots: Vec<IVec> = (2..20)
        .flat_map(|d| [home.offset(d, 0), home.offset(-d, 0), home.offset(0, d), home.offset(0, -d)])
        .filter(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none())
        .take(2)
        .collect();
    let cob = s.world.spawn_fixture_of(wall, spots[0], false, Some(def(&s, "primitive:clay"))).unwrap();
    let branch = s.world.spawn_fixture_of(wall, spots[1], false, Some(def(&s, "primitive:branches"))).unwrap();
    let (c, b) = (s.world.stat(cob, "insulation").unwrap(), s.world.stat(branch, "insulation").unwrap());
    assert!(c > 2.0 * b, "cob {c} against branches {b}");
}

/// The campfire is a station: a pot is fired in it from a bill, and it's
/// made of its clay.
#[test]
fn a_pot_is_fired_at_a_campfire() {
    let (mut s, founder) = alone(3);
    let home = s.world.pawn_pos(founder).unwrap();
    let def = |s: &Sim, id: &str| s.world.defs.thing_id(id).unwrap();
    let at = (2..10)
        .flat_map(|d| [home.offset(d, 0), home.offset(-d, 0), home.offset(0, d), home.offset(0, -d)])
        .find(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none())
        .expect("room for a fire");
    let fire = s.world.spawn_fixture_of(def(&s, "core:campfire"), at, false, None).unwrap();
    rim_sim::ai::complete_building(&mut s.world, fire);
    s.world.place_item(def(&s, "primitive:clay"), home, 3);
    let data = [
        (Key::Str("site".into()), Data::Int(fire.to_bits().get() as i64)),
        (Key::Str("recipe".into()), Data::Str("primitive:pot".into())),
    ];
    s.push(Command::ModEvent { name: "crafting:add_bill".into(), data: Some(Data::Table(data.into_iter().collect())) });
    let pot = def(&s, "primitive:pot");
    assert!(run_until(&mut s, 12_000, |s| count(s, "primitive:pot") == 1), "a pot is fired");
    let e = s.world.ecs.query::<(Entity, &Thing)>().iter().find(|(_, t)| t.def == pot).map(|(e, _)| e).unwrap();
    assert_eq!(s.world.made_of(e), Some(def(&s, "primitive:clay")));
}

/// A dead deer gives bone as well as its meat.
#[test]
fn a_deer_gives_bone() {
    let (mut s, founder) = alone(3);
    let home = s.world.pawn_pos(founder).unwrap();
    let deer = s.world.defs.creature_id("core:deer").unwrap();
    let at = (3..20)
        .flat_map(|d| [home.offset(d, 0), home.offset(-d, 0), home.offset(0, d), home.offset(0, -d)])
        .find(|&p| s.world.map.passable(p) && s.world.map.item_at(p).is_none())
        .expect("room for a deer");
    let e = s.world.spawn_pawn(deer, rim_sim::world::Faction::Wild, at, None);
    s.world.ecs.get::<&mut rim_sim::world::Pawn>(e).unwrap().dead = true;
    s.step();
    assert_eq!(count(&s, "primitive:bone"), 4);
    assert_eq!(count(&s, "core:raw_meat"), 35, "and all its meat still");
}

/// Bone knaps like flint, only worse: the same hand axe, made of bone,
/// wears out sooner and works slower.
#[test]
fn a_bone_hand_axe_is_worse_than_a_flint_one() {
    let axe_of = |material: &str| {
        let (mut s, founder) = alone(3);
        let home = s.world.pawn_pos(founder).unwrap();
        let def = |s: &Sim, id: &str| s.world.defs.thing_id(id).unwrap();
        let at = (2..10)
            .flat_map(|d| [home.offset(d, 0), home.offset(-d, 0), home.offset(0, d), home.offset(0, -d)])
            .find(|&p| {
                s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none()
            })
            .expect("room for a spot");
        let spot = s.world.spawn_fixture_of(def(&s, "crafting:spot"), at, false, None).unwrap();
        rim_sim::ai::complete_building(&mut s.world, spot);
        s.world.place_item(def(&s, material), home, 2);
        s.world.place_item(def(&s, "primitive:hammerstone"), home, 1);
        let data = [
            (Key::Str("site".into()), Data::Int(spot.to_bits().get() as i64)),
            (Key::Str("recipe".into()), Data::Str("primitive:hand_axe".into())),
        ];
        s.push(Command::ModEvent {
            name: "crafting:add_bill".into(),
            data: Some(Data::Table(data.into_iter().collect())),
        });
        let axe = def(&s, "primitive:hand_axe");
        assert!(run_until(&mut s, 12_000, |s| count(s, "primitive:hand_axe") == 1), "knapped from {material}");
        let (e, hp) =
            s.world.ecs.query::<(Entity, &Thing)>().iter().find(|(_, t)| t.def == axe).map(|(e, t)| (e, t.hp)).unwrap();
        assert_eq!(s.world.made_of(e), Some(def(&s, material)));
        (hp, s.world.tool_speed(e))
    };
    let ((bone_hp, bone_speed), (flint_hp, flint_speed)) = (axe_of("primitive:bone"), axe_of("primitive:flint"));
    assert_eq!((bone_hp, flint_hp), (48, 60), "bone's hp factor is 0.8");
    assert!(bone_speed < flint_speed, "bone {bone_speed} against flint {flint_speed}");
}

/// A first bed of grass: six fibre, no branches, and it's a bed.
#[test]
fn a_grass_pallet_is_a_bed_of_fibre() {
    let (mut s, founder) = alone(3);
    let home = s.world.pawn_pos(founder).unwrap();
    let pallet = s.world.defs.thing_id("primitive:pallet").unwrap();
    assert!(s.world.defs.thing(pallet).bed.as_ref().is_some_and(|b| b.rest_rate < 1.8), "a bed, poorer than core's");
    let at = (2..10)
        .flat_map(|d| [home.offset(d, 0), home.offset(-d, 0), home.offset(0, d), home.offset(0, -d)])
        .find(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none())
        .expect("room for a pallet");
    s.world.place_item(s.world.defs.thing_id("primitive:fibre").unwrap(), home, 6);
    s.push(Command::Build { thing: pallet, stuff: None, a: at, b: at });
    let built = |s: &Sim| s.world.map.fixture_at(at).is_some_and(|f| s.world.ecs.get::<&Blueprint>(f).is_err());
    assert!(run_until(&mut s, 6_000, built), "built from the fibre");
    assert_eq!(count(&s, "primitive:fibre"), 0);
}

/// With the plugin removed, core plays exactly as it did (DESIGN.md §5).
#[test]
fn core_alone_is_untouched() {
    let s = Sim::build(&common::mods(), 3, &|m| m == "core", 250).unwrap();
    let d = &s.world.defs;
    assert_eq!(d.thing(d.thing_id("tree_oak").unwrap()).harvest.len(), 1);
    assert!(d.thing(d.thing_id("tree_oak").unwrap()).harvest[0].requires.is_empty(), "chopped bare-handed");
    assert!(d.thing(d.thing_id("granite").unwrap()).harvest[0].requires.is_empty(), "mined bare-handed");
    let deer = d.creature(d.creature_id("deer").unwrap());
    assert_eq!(deer.butcher_r, vec![(d.thing_id("raw_meat").unwrap(), 35)], "a deer is only meat");
    let wood = d.thing_id("wood").unwrap();
    assert_eq!(d.thing(d.thing_id("campfire").unwrap()).build.as_ref().unwrap().cost_r, vec![(wood, 15)]);
}
