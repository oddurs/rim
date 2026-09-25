use rim_sim::world::{Blueprint, Thing};
use rim_sim::{Command, Sim, TICKS_PER_DAY};
use std::path::Path;

fn sim(seed: u64) -> Sim {
    Sim::new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods"), seed).expect("mods load")
}

/// The first-hour loop: chop trees, haul wood, raise walls.
#[test]
fn warrior_chops_and_builds() {
    let mut s = sim(3);
    let d = &s.world.defs;
    let (chop, wall) = (d.lookup("designation", "chop").unwrap(), d.thing_id("wall").unwrap());
    let wood = d.thing_id("wood").unwrap();
    let c = s.world.colony_center().unwrap();
    s.push(Command::Designate { designation: chop, a: c.offset(-15, -15), b: c.offset(15, 15) });
    // Find a free row of four open cells near the start.
    let m = &s.world.map;
    let free = |p: rim_sim::IVec| m.passable(p) && m.fixture_at(p).is_none();
    let row = (1..20)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))))
        .find(|&p| (0..4).all(|i| free(p.offset(i, 0))))
        .expect("no open ground near start");
    s.push(Command::Build { stuff: Some(wood), thing: wall, a: row, b: row.offset(3, 0) });
    let placed = 4;
    for _ in 0..TICKS_PER_DAY * 2 {
        s.step();
    }
    let w = &s.world;
    let built = w.ecs.query::<&Thing>().without::<&Blueprint>().iter().filter(|t| t.def == wall).count();
    let msgs: Vec<_> = w.messages.iter().map(|m| m.text.clone()).collect();
    assert!(built > 0, "no walls built out of {placed} blueprints; messages: {msgs:?}");
}

#[test]
fn shipped_mods_load_and_patch_applies() {
    let s = sim(1);
    assert_eq!(s.mods.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["core", "weather", "wildlife_plus"]);
    let d = &s.world.defs;
    assert!(d.creature_id("boar").is_some(), "plugin creature missing");
    let bush = d.thing(d.thing_id("berry_bush").unwrap());
    assert_eq!(bush.harvest[0].regrow_days, 1.5, "plugin patch not applied");
    assert!(s.warnings.is_empty(), "unexpected warnings: {:?}", s.warnings);
}
