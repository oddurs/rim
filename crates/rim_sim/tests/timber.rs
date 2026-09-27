//! The timber plugin (DESIGN.md §4f): planks hewn from logs with an axe,
//! and what they make, with no iron installed.

mod common;

use rim_sim::data::{Data, Key};
use rim_sim::hecs::Entity;
use rim_sim::world::{Blueprint, Store};
use rim_sim::{Command, IVec, Sim};

fn open_near(sim: &Sim, from: IVec, w: i32) -> IVec {
    (3..40)
        .flat_map(|r| [from.offset(r, 0), from.offset(-r, 0), from.offset(0, r), from.offset(0, -r)])
        .find(|&o| {
            (0..w).all(|dx| {
                let p = o.offset(dx, 0);
                sim.world.map.passable(p) && sim.world.map.item_at(p).is_none() && sim.world.map.fixture_at(p).is_none()
            })
        })
        .expect("open ground")
}

fn stands(sim: &Sim, p: IVec, id: &str) -> Option<Entity> {
    let def = sim.world.defs.thing_id(id)?;
    sim.world
        .map
        .fixture_at(p)
        .filter(|&f| sim.world.ecs.get::<&Blueprint>(f).is_err() && sim.world.thing(f).is_some_and(|t| t.def == def))
}

#[test]
fn without_iron_a_colony_hews_planks_and_builds_a_plank_wall_and_a_crate() {
    let dir = common::test_mods("timber-alone", &["core", "crafting", "primitive", "timber"], &[]);
    let mut sim = Sim::with_mods(&dir, 7, &|_| true).unwrap();
    let defs = sim.world.defs.clone();
    assert!(defs.thing_id("iron:nails").is_none(), "no iron here");
    assert_eq!(defs.thing(defs.thing_id("core:wood").unwrap()).label, "logs", "wood reads as logs");
    let pawn = sim.world.colonists().next().unwrap();
    let home = sim.world.pawn_pos(pawn).unwrap();
    // A crafting spot, an axe, and logs to hew.
    let spot_at = open_near(&sim, home, 1);
    let spot = sim.world.spawn_fixture_of(defs.thing_id("crafting:spot").unwrap(), spot_at, false, None).unwrap();
    let flint = defs.thing_id("primitive:flint");
    sim.world.place_item_of(defs.thing_id("primitive:hand_axe").unwrap(), home, 1, flint);
    sim.world.place_item(defs.thing_id("core:wood").unwrap(), home.offset(1, 1), 40);
    let bill = |name: &str, pairs: Vec<(&str, Data)>| Command::ModEvent {
        name: name.into(),
        data: Some(Data::Table(pairs.into_iter().map(|(k, v)| (Key::Str(k.into()), v)).collect())),
    };
    let site = Data::Int(spot.to_bits().get() as i64);
    sim.push(bill(
        "crafting:add_bill",
        vec![("site", site.clone()), ("recipe", Data::Str("timber:hew_planks".into()))],
    ));
    sim.step();
    // Enough for a wall and a crate: until there are twelve.
    let Some(Data::Table(state)) = sim.world.data.get("crafting:bills") else { panic!("no bills") };
    let id = state
        .values()
        .find(|st| st.get("site") == Some(&site))
        .and_then(|st| st.get("bills"))
        .and_then(|b| b.get_index(1))
        .and_then(|b| b.get("id"))
        .cloned()
        .expect("the bill");
    let until = vec![("site", site), ("bill", id), ("mode", Data::Str("until".into())), ("target", Data::Int(12))];
    sim.push(bill("crafting:set_bill", until));
    // Plans for a plank wall and a crate, out of planks.
    let planks = defs.thing_id("timber:planks");
    let wall_at = open_near(&sim, home.offset(0, 6), 1);
    let crate_at = open_near(&sim, home.offset(6, 0), 1);
    sim.push(Command::Build {
        thing: defs.thing_id("timber:plank_wall").unwrap(),
        stuff: planks,
        a: wall_at,
        b: wall_at,
        facing: 0,
    });
    sim.push(Command::Build {
        thing: defs.thing_id("timber:crate").unwrap(),
        stuff: planks,
        a: crate_at,
        b: crate_at,
        facing: 0,
    });
    let mut done = false;
    for _ in 0..(3 * rim_sim::TICKS_PER_DAY) {
        sim.step();
        if stands(&sim, wall_at, "timber:plank_wall").is_some() && stands(&sim, crate_at, "timber:crate").is_some() {
            done = true;
            break;
        }
    }
    assert!(done, "planks were hewn, and the wall and the crate went up: {:?}", sim.world.messages.last());
    let c = stands(&sim, crate_at, "timber:crate").unwrap();
    assert_eq!(sim.world.ecs.get::<&Store>(c).map(|s| s.slots.len()).ok(), Some(4), "a crate with its four slots");
    let _ = std::fs::remove_dir_all(dir);
}
