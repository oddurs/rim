//! Carried things keep what they're made of, and their hp: a flint axe
//! hauled to a stockpile or brought to a work order is still that axe.

mod common;

use rim_sim::data::{Data, Key};
use rim_sim::defs::DefId;
use rim_sim::hecs::Entity;
use rim_sim::world::{Lot, MadeOf, Thing};
use rim_sim::{Command, IVec, Sim};
use std::path::PathBuf;

const DEFS: &str = r##"
[[work_type]]
id = "craft"
label = "Craft"

[[thing]]
id = "bench"
label = "bench"
color = "#806040"
category = "building"
look.layers = [{ draw = "fill" }]
build = { cost = [{ thing = "core:wood", count = 1 }], work = 10 }

[[thing]]
id = "flint"
label = "flint"
color = "#445566"
category = "item"
look.layers = [{ draw = "fill" }]
stuff = { categories = ["lithic"], factors = { hp = 0.5 } }

[[thing]]
id = "bone"
label = "bone"
color = "#ddddcc"
category = "item"
look.layers = [{ draw = "fill" }]
stuff = { categories = ["lithic"], factors = { hp = 0.8 } }

[[thing]]
id = "axe"
label = "axe"
color = "#aabbcc"
category = "item"
look.layers = [{ draw = "fill" }]
hp = 80
stack_limit = 5

[[thing]]
id = "pick"
label = "pick"
color = "#8899aa"
category = "item"
look.layers = [{ draw = "fill" }]
hp = 80
tool = { tags = ["pounding"] }
"##;

/// A work order that takes an axe, and says what the axe was made of.
const SCRIPT: &str = r##"
rim.on("kit:make", function(e)
    rim.post_order(e.site, {
        label = "rehaft", work_type = "craft", work = 10,
        needs = { { thing = "axe", count = 1 } },
    })
end)
rim.on("kit:picks", function(e)
    rim.post_order(e.site, {
        label = "picks", work_type = "craft", work = 5000,
        needs = { { thing = "pick", count = 2 } },
    })
end)
rim.on("kit:cancel", function(e)
    rim.cancel_order(e.site)
end)
rim.on("order_done", function(e)
    if e.owner ~= "kit" then return end
    rim.set_data("made_of", e.inputs[1].made_of)
    rim.set_data("stuff", e.stuff)
end)
"##;

fn kit(name: &str) -> PathBuf {
    common::test_mods(name, &["core"], &[("kit", &[("defs/kit.toml", DEFS), ("scripts/kit.luau", SCRIPT)])])
}

/// One colonist and open ground a few cells off.
fn colony(name: &str) -> (Sim, Entity, IVec) {
    let mut s = Sim::new(&kit(name), 5).unwrap_or_else(|e| panic!("mods load: {e}"));
    s.step();
    let founder = s.world.colonists().next().expect("a founder");
    for e in s.world.pawns.clone() {
        if e != founder {
            let _ = s.world.ecs.despawn(e);
        }
    }
    s.world.pawns.retain(|&e| e == founder);
    let home = s.world.pawn_pos(founder).unwrap();
    let open = |s: &Sim, o: IVec| {
        (0..3).all(|x| {
            (0..3).all(|y| {
                let p = o.offset(x, y);
                s.world.map.passable(p) && s.world.map.item_at(p).is_none() && s.world.map.fixture_at(p).is_none()
            })
        })
    };
    let site = (3..40)
        .flat_map(|r| [home.offset(r, 0), home.offset(-r, 0), home.offset(0, r), home.offset(0, -r)])
        .find(|&o| open(&s, o))
        .expect("open ground");
    (s, founder, site)
}

fn send(s: &mut Sim, name: &str, site: Entity) {
    let data = [(Key::Str("site".into()), Data::Int(site.to_bits().get() as i64))].into_iter().collect();
    s.push(Command::ModEvent { name: name.into(), data: Some(Data::Table(data)) });
}

fn def(s: &Sim, id: &str) -> DefId {
    s.world.defs.thing_id(id).unwrap()
}

/// Every stack of axes: where, how many, made of what, at what hp.
fn axes(s: &Sim) -> Vec<(IVec, u32, Option<DefId>, i32)> {
    let axe = def(s, "kit:axe");
    let mut v: Vec<_> = s
        .world
        .ecs
        .query::<(&Thing, Option<&MadeOf>)>()
        .iter()
        .filter(|(t, _)| t.def == axe)
        .map(|(t, m)| (t.pos, t.count, m.map(|m| m.0), t.hp))
        .collect();
    v.sort_by_key(|a| (a.2, a.0.x, a.0.y));
    v
}

fn in_zone(s: &Sim, p: IVec) -> bool {
    s.world.zones.at(&s.world.map, p).is_some()
}

#[test]
fn a_hauled_flint_axe_is_still_a_flint_axe_with_its_hp() {
    let (mut s, founder, site) = colony("carry-haul");
    let (axe, flint) = (def(&s, "kit:axe"), def(&s, "kit:flint"));
    let from = s.world.pawn_pos(founder).unwrap();
    s.world.place_item_of(axe, from, 1, Some(flint));
    let e = s.world.ecs.query::<(Entity, &Thing)>().iter().find(|(_, t)| t.def == axe).map(|(e, _)| e).unwrap();
    s.world.ecs.get::<&mut Thing>(e).unwrap().hp = 17;
    s.push(Command::Stockpile { a: site, b: site.offset(2, 2), zone: None });
    for _ in 0..3_000 {
        s.step();
    }
    let a = axes(&s);
    assert_eq!(a.len(), 1);
    assert!(in_zone(&s, a[0].0), "hauled: {a:?}");
    assert_eq!((a[0].2, a[0].3), (Some(flint), 17), "still flint, still worn");
}

#[test]
fn two_materials_never_merge_by_hauling() {
    let (mut s, founder, site) = colony("carry-merge");
    let (axe, flint, bone) = (def(&s, "kit:axe"), def(&s, "kit:flint"), def(&s, "kit:bone"));
    let from = s.world.pawn_pos(founder).unwrap();
    s.world.place_item_of(axe, from, 2, Some(flint));
    s.world.place_item_of(axe, from, 2, Some(bone));
    s.push(Command::Stockpile { a: site, b: site.offset(2, 2), zone: None });
    for _ in 0..4_000 {
        s.step();
    }
    let a = axes(&s);
    assert!(a.iter().all(|x| in_zone(&s, x.0)), "all hauled: {a:?}");
    let by: Vec<_> = a.iter().map(|x| (x.1, x.2)).collect();
    assert_eq!(by, vec![(2, Some(flint)), (2, Some(bone))], "a stack of each");
}

#[test]
fn an_input_brought_to_an_order_says_what_it_was_made_of() {
    let (mut s, founder, site) = colony("carry-order");
    let (axe, flint) = (def(&s, "kit:axe"), def(&s, "kit:flint"));
    let bench = s.world.spawn_fixture_of(def(&s, "kit:bench"), site, false, None).unwrap();
    rim_sim::ai::complete_building(&mut s.world, bench);
    let from = s.world.pawn_pos(founder).unwrap();
    s.world.place_item_of(axe, from, 1, Some(flint));
    send(&mut s, "kit:make", bench);
    for _ in 0..4_000 {
        s.step();
        if s.world.data.contains_key("kit:made_of") {
            break;
        }
    }
    assert_eq!(s.world.data.get("kit:made_of"), Some(&Data::Str("kit:flint".into())));
    assert_eq!(s.world.data.get("kit:stuff"), Some(&Data::Str("kit:flint".into())), "what it makes is flint too");
}

/// Two tools worn differently, brought to one order and put back when it's
/// cancelled, each come back as they were, and are tools again.
#[test]
fn tools_brought_to_an_order_come_back_as_they_were() {
    let (mut s, founder, site) = colony("carry-cancel");
    let (pick, flint) = (def(&s, "kit:pick"), def(&s, "kit:flint"));
    let bench = s.world.spawn_fixture_of(def(&s, "kit:bench"), site, false, None).unwrap();
    rim_sim::ai::complete_building(&mut s.world, bench);
    let from = s.world.pawn_pos(founder).unwrap();
    s.world.place_item_of(pick, from, 1, Some(flint));
    s.world.place_item_of(pick, from, 1, Some(flint));
    let picks = |s: &Sim| -> Vec<(Entity, i32)> {
        let mut v: Vec<_> = s
            .world
            .ecs
            .query::<(Entity, &Thing)>()
            .iter()
            .filter(|(_, t)| t.def == pick)
            .map(|(e, t)| (e, t.hp))
            .collect();
        v.sort_by_key(|p| p.1);
        v
    };
    for (i, (e, _)) in picks(&s).into_iter().enumerate() {
        s.world.ecs.get::<&mut Thing>(e).unwrap().hp = [10, 50][i];
    }
    send(&mut s, "kit:picks", bench);
    let brought = |s: &Sim| s.world.ecs.get::<&rim_sim::world::Order>(bench).is_ok_and(|o| o.missing().is_none());
    let mut ok = false;
    for _ in 0..6_000 {
        s.step();
        if brought(&s) {
            ok = true;
            break;
        }
    }
    assert!(ok, "both picks brought");
    assert!(picks(&s).is_empty(), "off the map while the order holds them");
    send(&mut s, "kit:cancel", bench);
    s.step();
    let back = picks(&s);
    assert_eq!(back.iter().map(|p| p.1).collect::<Vec<_>>(), vec![10, 50], "each with its own wear");
    assert!(back.iter().all(|(e, _)| s.world.tools.contains(e)), "and tools again");
    assert!(back.iter().all(|&(e, _)| s.world.made_of(e) == Some(flint)));
}

/// Saves from before this carried `(def, count)`: they still load, as
/// plain things.
#[test]
fn an_old_carry_still_loads() {
    let old = rmp_serde::to_vec_named(&Some((3 as DefId, 7u32))).unwrap();
    let lot: Option<Lot> = rmp_serde::from_slice(&old).unwrap();
    assert_eq!(lot, Some(Lot::new(3, 7)));
    let now = Lot { def: 3, count: 7, made_of: Some(2), hp: Some(40) };
    let back: Lot = rmp_serde::from_slice(&rmp_serde::to_vec_named(&now).unwrap()).unwrap();
    assert_eq!(back, now);
}
