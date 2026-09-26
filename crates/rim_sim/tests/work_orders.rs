//! Work orders (DESIGN.md §4e): a mod posts "bring these, then work
//! here", colonists do it, and the mod hears what went in.

mod common;

use rim_sim::data::{Data, Key};
use rim_sim::hecs::Entity;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{MadeOf, Order, Thing};
use rim_sim::{Command, IVec, Sim};
use std::path::{Path, PathBuf};

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
id = "chip"
label = "chip"
color = "#445566"
category = "item"
look.layers = [{ draw = "fill" }]
stack_limit = 10
tags = ["shard"]
stuff = { categories = ["lithic"], factors = { hp = 0.5 } }

[[thing]]
id = "slab"
label = "slab"
color = "#667788"
category = "item"
look.layers = [{ draw = "fill" }]
stack_limit = 10
tags = ["shard"]

[[thing]]
id = "blade"
label = "blade"
color = "#aabbcc"
category = "item"
look.layers = [{ draw = "fill" }]
hp = 80
stack_limit = 10

[[thing]]
id = "hammer"
label = "hammer"
color = "#555555"
category = "item"
look.layers = [{ draw = "fill" }]
tool = { tags = ["pounding"] }
"##;

/// Make a blade: three shards and two wood, 100 work, maybe a tool. When
/// it's done, a blade of the first material appears at the bench.
const SCRIPT: &str = r##"
rim.on("bench:make", function(e)
    rim.post_order(e.site, {
        label = "blade", work_type = "craft", work = 100, requires = e.requires,
        needs = { { tag = "shard", count = 3 }, { thing = "core:wood", count = 2 } },
    })
end)
rim.on("bench:cancel", function(e)
    rim.set_data("cancelled", rim.cancel_order(e.site))
end)
rim.on("bench:count", function(e)
    rim.set_data("shards", rim.count_items({ tag = "shard" }))
    rim.set_data("wood", rim.count_items({ thing = "core:wood" }))
end)
rim.on("bench:post_on", function(e)
    rim.post_order(e.site, { label = "x", work_type = "craft", work = 1, needs = { { thing = "core:wood", count = 1 } } })
end)
rim.on("order_lost", function(e)
    rim.set_data("lost", e.label)
end)
rim.on("order_done", function(e)
    if e.owner ~= "bench" then return end
    rim.spawn_item("blade", e.x, e.y, 1, e.stuff)
    rim.set_data("inputs", #e.inputs)
end)
"##;

fn bench_mods(name: &str) -> PathBuf {
    common::test_mods(name, &["core"], &[("bench", &[("defs/bench.toml", DEFS), ("scripts/bench.luau", SCRIPT)])])
}

/// One colonist, a finished bench beside them, and its entity.
fn world(dir: &Path) -> (Sim, Entity, Entity) {
    let mut s = Sim::new(dir, 3).unwrap_or_else(|e| panic!("mods load: {e}"));
    s.step();
    let founder = s.world.colonists().next().expect("a founder");
    for e in s.world.pawns.clone() {
        if e != founder {
            let _ = s.world.ecs.despawn(e);
        }
    }
    s.world.pawns.retain(|&e| e == founder);
    let home = s.world.pawn_pos(founder).unwrap();
    let at = (2..8)
        .flat_map(|d| [home.offset(d, 0), home.offset(-d, 0), home.offset(0, d), home.offset(0, -d)])
        .find(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none())
        .expect("room for a bench");
    let bench = s.world.defs.thing_id("bench:bench").unwrap();
    let e = s.world.spawn_fixture_of(bench, at, false, None).expect("placed");
    rim_sim::ai::complete_building(&mut s.world, e);
    (s, founder, e)
}

fn put(s: &mut Sim, id: &str, n: u32, near: IVec) {
    let d = s.world.defs.thing_id(id).unwrap();
    s.world.place_item(d, near, n);
}

fn send(s: &mut Sim, name: &str, site: Entity, extra: &[(&str, Data)]) {
    let mut t: std::collections::BTreeMap<Key, Data> =
        [(Key::Str("site".into()), Data::Int(site.to_bits().get() as i64))].into_iter().collect();
    for (k, v) in extra {
        t.insert(Key::Str((*k).into()), v.clone());
    }
    s.push(Command::ModEvent { name: name.into(), data: Some(Data::Table(t)) });
}

fn blades(s: &Sim) -> Vec<(Entity, Option<rim_sim::defs::DefId>, i32)> {
    let blade = s.world.defs.thing_id("bench:blade").unwrap();
    s.world
        .ecs
        .query::<(Entity, &Thing, Option<&MadeOf>)>()
        .iter()
        .filter(|(_, t, _)| t.def == blade)
        .map(|(e, t, m)| (e, m.map(|m| m.0), t.hp))
        .collect()
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

fn count(s: &Sim, id: &str) -> u32 {
    let d = s.world.defs.thing_id(id).unwrap();
    s.world.ecs.query::<&Thing>().iter().filter(|t| t.def == d).map(|t| t.count).sum()
}

#[test]
fn an_order_is_supplied_by_tag_worked_and_heard() {
    let (mut s, founder, bench) = world(&bench_mods("orders-make"));
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "bench:chip", 2, home);
    put(&mut s, "bench:slab", 1, home);
    put(&mut s, "core:wood", 2, home);
    let wood_before = count(&s, "core:wood");
    send(&mut s, "bench:make", bench, &[]);
    assert!(run_until(&mut s, 6_000, |s| !blades(s).is_empty()), "a blade is made");
    assert!(s.world.ecs.get::<&Order>(bench).is_err(), "the order is done");
    assert_eq!((count(&s, "bench:chip"), count(&s, "bench:slab")), (0, 0), "shards used up, by tag");
    assert_eq!(count(&s, "core:wood"), wood_before - 2);
    let chip = s.world.defs.thing_id("bench:chip").unwrap();
    let (_, made_of, hp) = blades(&s)[0];
    assert_eq!(made_of, Some(chip), "made of the first material that went in");
    assert_eq!(hp, 40, "80 hp at chip's 0.5");
    assert_eq!(s.world.data.get("bench:inputs"), Some(&Data::Int(3)), "chip, slab and wood");
}

#[test]
fn an_order_that_needs_a_tool_waits_for_one() {
    let (mut s, founder, bench) = world(&bench_mods("orders-tool"));
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "bench:chip", 3, home);
    put(&mut s, "core:wood", 2, home);
    let pounding = Data::Table([(Key::Int(1), Data::Str("pounding".into()))].into_iter().collect());
    // There's no hammer yet, but the tag is some tool's, so the order posts.
    send(&mut s, "bench:make", bench, &[("requires", pounding.clone())]);
    s.step();
    assert!(s.world.ecs.get::<&Order>(bench).is_ok(), "hammer is a tool def, so the tag is known");
    assert!(!run_until(&mut s, 4_000, |s| !blades(s).is_empty()), "no hammer, no blade");
    assert_eq!(s.world.ecs.get::<&Order>(bench).unwrap().missing(), None, "everything was brought");
    put(&mut s, "bench:hammer", 1, home);
    assert!(run_until(&mut s, 4_000, |s| !blades(s).is_empty()), "with a hammer, a blade");
}

#[test]
fn cancelling_puts_back_what_was_brought() {
    let (mut s, founder, bench) = world(&bench_mods("orders-cancel"));
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "bench:chip", 3, home);
    send(&mut s, "bench:make", bench, &[]);
    assert!(
        run_until(&mut s, 4_000, |s| s.world.ecs.get::<&Order>(bench).is_ok_and(|o| o.needs[0].have() == 3)),
        "the shards arrive; the wood never does"
    );
    assert_eq!(count(&s, "bench:chip"), 0);
    send(&mut s, "bench:cancel", bench, &[]);
    s.step();
    assert_eq!(s.world.data.get("bench:cancelled"), Some(&Data::Bool(true)));
    assert!(s.world.ecs.get::<&Order>(bench).is_err());
    assert_eq!(count(&s, "bench:chip"), 3, "put back down at the bench");
}

#[test]
fn a_half_supplied_order_survives_a_save() {
    let dir = bench_mods("orders-save");
    let (mut s, founder, bench) = world(&dir);
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "bench:chip", 3, home);
    send(&mut s, "bench:make", bench, &[]);
    assert!(run_until(&mut s, 4_000, |s| s.world.ecs.get::<&Order>(bench).is_ok_and(|o| o.needs[0].have() > 0)));
    let snap = Snapshot::capture(&s);
    let back = snap.restore(&dir, &|_| true).unwrap_or_else(|e| panic!("restores: {e}"));
    let o = back.world.ecs.get::<&Order>(bench).expect("still posted");
    assert_eq!(
        (o.owner.as_str(), o.needs[0].have()),
        ("bench", s.world.ecs.get::<&Order>(bench).unwrap().needs[0].have())
    );
    drop(o);
    assert_eq!(Snapshot::capture(&back).hash(), snap.hash());
}

#[test]
fn items_are_counted_by_thing_and_by_tag() {
    let (mut s, founder, bench) = world(&bench_mods("orders-count"));
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "bench:chip", 4, home);
    put(&mut s, "bench:slab", 2, home);
    let wood = count(&s, "core:wood");
    send(&mut s, "bench:count", bench, &[]);
    s.step();
    assert_eq!(s.world.data.get("bench:shards"), Some(&Data::Int(6)));
    assert_eq!(s.world.data.get("bench:wood"), Some(&Data::Int(wood as i64)));
}

/// Stacks of one thing in two materials stay apart, and a plain one
/// doesn't merge into either.
#[test]
fn stacks_keep_their_material() {
    let (mut s, founder, _) = world(&bench_mods("orders-stuff"));
    let home = s.world.pawn_pos(founder).unwrap();
    let (chip, blade) = (s.world.defs.thing_id("bench:chip").unwrap(), s.world.defs.thing_id("bench:blade").unwrap());
    let wood = s.world.defs.thing_id("core:wood").unwrap();
    s.world.place_item_of(blade, home, 2, Some(chip));
    s.world.place_item_of(blade, home, 1, Some(wood));
    s.world.place_item_of(blade, home, 3, Some(chip));
    s.world.place_item(blade, home, 1);
    let mut stacks: Vec<(Option<rim_sim::defs::DefId>, u32)> = s
        .world
        .ecs
        .query::<(&Thing, Option<&MadeOf>)>()
        .iter()
        .filter(|(t, _)| t.def == blade)
        .map(|(t, m)| (m.map(|m| m.0), t.count))
        .collect();
    let mut want = vec![(None, 1), (Some(wood), 1), (Some(chip), 5)];
    stacks.sort();
    want.sort();
    assert_eq!(stacks, want);
}

/// A site torn down mid-order keeps what was brought on the ground, and
/// the mod that posted it hears.
#[test]
fn a_lost_site_puts_back_what_was_brought() {
    let (mut s, founder, bench) = world(&bench_mods("orders-lost"));
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "bench:chip", 3, home);
    send(&mut s, "bench:make", bench, &[]);
    assert!(run_until(&mut s, 4_000, |s| s.world.ecs.get::<&Order>(bench).is_ok_and(|o| o.needs[0].have() == 3)));
    s.world.despawn_thing(bench);
    s.step();
    assert_eq!(count(&s, "bench:chip"), 3, "back on the ground");
    assert_eq!(s.world.data.get("bench:lost"), Some(&Data::Str("blade".into())));
}

/// Only something built can be a site: an item could be eaten or carried off.
#[test]
fn an_item_is_no_site() {
    let (mut s, founder, _) = world(&bench_mods("orders-site"));
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "bench:chip", 1, home);
    let chip = s.world.defs.thing_id("bench:chip").unwrap();
    let item = s.world.ecs.query::<(Entity, &Thing)>().iter().find(|(_, t)| t.def == chip).map(|(e, _)| e).unwrap();
    send(&mut s, "bench:post_on", item, &[]);
    s.step();
    assert!(s.world.ecs.get::<&Order>(item).is_err());
    assert!(
        s.world.messages.iter().any(|m| m.text.contains("a site is a building or station")),
        "and the mod is told why"
    );
}
