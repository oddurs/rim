//! Tailoring's first step (mods/primitive): a hunted animal gives hides,
//! and a colonist with an edge scrapes a hide into leather.

mod common;

use rim_sim::data::{Data, Key};
use rim_sim::world::{Faction, Pawn, Thing, NEED_MAX};
use rim_sim::{Command, IVec, Sim};

fn count(s: &Sim, thing: &str) -> u32 {
    let d = s.world.defs.thing_id(thing).unwrap();
    s.world.ecs.query::<&Thing>().iter().filter(|t| t.def == d).map(|t| t.count).sum()
}

#[test]
fn a_butchered_deer_gives_hides_and_a_hide_becomes_leather() {
    let dir = common::test_mods("leather", &["core", "crafting", "primitive"], &[]);
    let mut s = Sim::new(&dir, 4).unwrap_or_else(|e| panic!("mods load: {e}"));
    s.step();
    let tanner = s.world.colonists().next().expect("a founder");
    for e in s.world.pawns.clone() {
        if e != tanner && s.world.ecs.get::<&Pawn>(e).is_ok_and(|p| p.faction == Faction::Player) {
            let _ = s.world.ecs.despawn(e);
        }
    }
    s.world.pawns.retain(|&e| s.world.ecs.get::<&Pawn>(e).is_ok());
    common::hands(&mut s);
    s.world.ecs.get::<&mut Pawn>(tanner).unwrap().needs.iter_mut().for_each(|n| n.1 = NEED_MAX);
    let home = s.world.pawn_pos(tanner).unwrap();

    // A deer dies: its hides fall with its meat and bones.
    let hides = count(&s, "primitive:hide");
    let deer = s.world.defs.lookup("creature", "core:deer").unwrap();
    let d = s.world.spawn_pawn(deer, Faction::Wild, home.offset(3, 0), None);
    s.world.ecs.get::<&mut Pawn>(d).unwrap().dead = true;
    s.step();
    assert_eq!(count(&s, "primitive:hide"), hides + 3, "a deer's hide makes three");

    // Scraped at a crafting spot with a flint flake: leather.
    let open =
        |p: IVec| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none();
    let at = (2..10).map(|k| home.offset(-k, 0)).find(|&p| open(p)).expect("room for a spot");
    let spot = s.world.defs.thing_id("crafting:spot").unwrap();
    let spot = s.world.spawn_fixture_of(spot, at, false, None).expect("placed");
    rim_sim::ai::complete_building(&mut s.world, spot);
    let flake = s.world.defs.thing_id("primitive:flake").unwrap();
    s.world.place_item(flake, home, 1);
    let pairs = [("site", Data::Int(spot.to_bits().get() as i64)), ("recipe", Data::Str("primitive:leather".into()))];
    let data = Data::Table(pairs.into_iter().map(|(k, v)| (Key::Str(k.into()), v)).collect());
    s.push(Command::ModEvent { name: "crafting:add_bill".into(), data: Some(data) });
    let made = (0..12_000).any(|_| {
        s.step();
        count(&s, "primitive:leather") >= 1
    });
    assert!(made, "a hide is scraped into leather");
    assert_eq!(count(&s, "primitive:hide"), hides + 2, "out of one hide");
}
