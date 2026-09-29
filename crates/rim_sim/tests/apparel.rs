//! Apparel: colonists wear what tailors make, dress for the cold and
//! undress in the warmth, and what they wear keeps the cold off.

mod common;

use rim_sim::data::{Data, Key};
use rim_sim::hecs::Entity;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{Faction, Pawn, Thing, NEED_MAX};
use rim_sim::{Command, IVec, Sim};

/// A mod's garment, as data alone.
const SCARF: &str = r##"
[[thing]]
id = "scarf"
label = "scarf"
color = "#c04040"
category = "item"
look.layers = [{ draw = "fill", x = 0.3, y = 0.3, w = 0.4, h = 0.4 }]
hp = 40
stack_limit = 1
apparel = { layer = "neck", warmth = 5.0, wear_per_day = 2.0 }
"##;

/// One colonist, fed and rested, in Hand, and where it stands.
fn alone(s: &mut Sim) -> (Entity, IVec) {
    s.step();
    let me = s.world.colonists().next().expect("a founder");
    for e in s.world.pawns.clone() {
        if e != me && s.world.ecs.get::<&Pawn>(e).is_ok_and(|p| p.faction == Faction::Player) {
            let _ = s.world.ecs.despawn(e);
        }
    }
    s.world.pawns.retain(|&e| s.world.ecs.get::<&Pawn>(e).is_ok());
    common::hands(s);
    s.world.ecs.get::<&mut Pawn>(me).unwrap().needs.iter_mut().for_each(|n| n.1 = NEED_MAX);
    (me, s.world.pawn_pos(me).unwrap())
}

fn feels(s: &mut Sim, v: f64) {
    let f = s.world.defs.lookup("field", "core:feels_like").unwrap() as usize;
    s.world.fields.set_ambient(f, Some(v));
}

fn worn(s: &Sim, e: Entity) -> Vec<Entity> {
    s.world.ecs.get::<&Pawn>(e).map(|p| p.worn.clone()).unwrap_or_default()
}

#[test]
fn a_colonist_dresses_for_the_cold_and_undresses_in_the_warmth() {
    let dir = common::test_mods("apparel-scarf", &["core"], &[("knit", &[("defs/knit.toml", SCARF)])]);
    let mut s = Sim::new(&dir, 3).expect("mods load");
    let (me, home) = alone(&mut s);
    let scarf = s.world.defs.thing_id("knit:scarf").unwrap();
    s.world.place_item(scarf, home.offset(3, 1), 1);
    // Mild: nothing to put on.
    feels(&mut s, 14.0);
    for _ in 0..300 {
        s.step();
    }
    assert!(worn(&s, me).is_empty(), "no scarf in the mild");
    // Cold: fetched and worn, off the map and out of the stock.
    feels(&mut s, 0.0);
    let on = (0..3000).any(|_| {
        s.step();
        !worn(&s, me).is_empty()
    });
    assert!(on, "a scarf in the cold");
    let g = worn(&s, me)[0];
    assert!(s.world.map.item_at(s.world.thing(g).unwrap().pos) != Some(g), "off the map");
    assert!((s.world.warmth_worn(&s.world.ecs.get::<&Pawn>(me).unwrap()) - 5.0).abs() < 1e-9);
    // Warm again: taken off, and back on the map.
    feels(&mut s, 22.0);
    let off = (0..600).any(|_| {
        s.step();
        worn(&s, me).is_empty()
    });
    assert!(off, "the scarf comes off in the warmth");
    let t = s.world.thing(g).expect("still a scarf");
    assert_eq!(s.world.map.item_at(t.pos), Some(g));
}

#[test]
fn what_is_worn_keeps_the_cold_off_and_is_saved_and_hashed() {
    let dir = common::test_mods("apparel-save", &["core"], &[("knit", &[("defs/knit.toml", SCARF)])]);
    let mut s = Sim::new(&dir, 5).expect("mods load");
    let (me, home) = alone(&mut s);
    let scarf = s.world.defs.thing_id("knit:scarf").unwrap();
    s.world.place_item(scarf, home, 1);
    let g = s.world.map.item_at(home).unwrap();
    let before = s.world.state_hash();
    let mut p = (*s.world.ecs.get::<&Pawn>(me).unwrap()).clone();
    s.world.put_on(me, &mut p, g);
    *s.world.ecs.get::<&mut Pawn>(me).unwrap() = p;
    assert_ne!(s.world.state_hash(), before, "the hash sees what's worn");
    let back = Snapshot::capture(&s).restore(&dir, &|_| true).expect("loads");
    assert_eq!(worn(&back, me), vec![g]);
    assert_eq!(back.world.state_hash(), s.world.state_hash());
    // Worn, it lies nowhere: not back on the cell it was picked up from.
    assert_eq!(back.world.map.item_at(home), None);
    assert_eq!(back.world.stock, s.world.stock);

    // At 7°, three below comfort: bare, the warmth need drains; in a scarf
    // (5° of warmth), it doesn't.
    let mut bare = Sim::new(&dir, 5).expect("mods load");
    let (bme, _) = alone(&mut bare);
    let (dressed, stripped) = (chill(&mut s, me), chill(&mut bare, bme));
    assert!(dressed > stripped, "dressed {dressed} bare {stripped}");
}

/// The warmth need after a while at 7°, from half full.
fn chill(s: &mut Sim, who: Entity) -> i32 {
    feels(s, 7.0);
    let d = s.world.defs.lookup("need", "core:warmth").unwrap();
    s.world.ecs.get::<&mut Pawn>(who).unwrap().needs.iter_mut().for_each(|n| n.1 = NEED_MAX / 2);
    for _ in 0..200 {
        rim_sim::systems::needs(&mut s.world);
    }
    s.world.ecs.get::<&Pawn>(who).unwrap().needs.iter().find(|n| n.0 == d).unwrap().1
}

#[test]
fn primitive_leather_makes_a_cloak() {
    let dir = common::test_mods("apparel-cloak", &["core", "crafting", "primitive"], &[]);
    let mut s = Sim::new(&dir, 4).unwrap_or_else(|e| panic!("mods load: {e}"));
    let (_, home) = alone(&mut s);
    let count = |s: &Sim, id: &str| {
        let d = s.world.defs.thing_id(id).unwrap();
        s.world.ecs.query::<&Thing>().iter().filter(|t| t.def == d).map(|t| t.count).sum::<u32>()
    };
    let open =
        |p: IVec| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none();
    let at = (2..10).map(|k| home.offset(-k, 0)).find(|&p| open(p)).expect("room for a spot");
    let spot = s.world.defs.thing_id("crafting:spot").unwrap();
    let spot = s.world.spawn_fixture_of(spot, at, false, None).expect("placed");
    rim_sim::ai::complete_building(&mut s.world, spot);
    let (flake, leather) =
        (s.world.defs.thing_id("primitive:flake").unwrap(), s.world.defs.thing_id("primitive:leather").unwrap());
    s.world.place_item(flake, home, 1);
    s.world.place_item(leather, home.offset(1, 1), 3);
    let pairs =
        [("site", Data::Int(spot.to_bits().get() as i64)), ("recipe", Data::Str("primitive:leather_cloak".into()))];
    let data = Data::Table(pairs.into_iter().map(|(k, v)| (Key::Str(k.into()), v)).collect());
    s.push(Command::ModEvent { name: "crafting:add_bill".into(), data: Some(data) });
    let made = (0..15_000).any(|_| {
        s.step();
        count(&s, "primitive:leather_cloak") >= 1
    });
    assert!(made, "three leather make a cloak");
    let cloak = s.world.defs.thing(s.world.defs.thing_id("primitive:leather_cloak").unwrap());
    assert!(cloak.apparel.as_ref().is_some_and(|a| a.warmth > 0.0), "and it's worn");
}
