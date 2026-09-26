//! The crafting plugin (mods/crafting): recipes, stations and bills, on
//! the engine's work orders.

mod common;

use rim_sim::data::{Data, Key};
use rim_sim::hecs::Entity;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{Blueprint, MadeOf, Order, Thing};
use rim_sim::{Command, IVec, Sim};
use std::path::{Path, PathBuf};

const DEFS: &str = r##"
[[thing]]
id = "chip"
label = "chip"
color = "#445566"
category = "item"
look.layers = [{ draw = "fill" }]
stack_limit = 20
tags = ["shard"]
stuff = { categories = ["lithic"], factors = { hp = 0.5 } }

[[thing]]
id = "blade"
label = "blade"
color = "#aabbcc"
category = "item"
look.layers = [{ draw = "fill" }]
hp = 80
stack_limit = 20

[[crafting.recipe]]
id = "blade"
label = "blade"
station = "crafting:hand"
inputs = [{ tag = "shard", count = 2 }]
outputs = [{ thing = "blade" }]
work = 60

[[crafting.recipe]]
id = "slow"
label = "slow blade"
station = "crafting:hand"
inputs = [{ tag = "shard", count = 2 }]
outputs = [{ thing = "blade" }]
work = 5000

[[crafting.recipe]]
id = "odd"
label = "odd"
station = "crafting:hand"
inputs = [{ thing = "chip", count = 1 }]
outputs = [{ thing = "blade" }]
work = 10
work_type = "nope"

[[thing]]
id = "hammer"
label = "hammer"
color = "#555555"
category = "item"
look.layers = [{ draw = "fill" }]
tool = { tags = ["pounding"] }

[[crafting.recipe]]
id = "hammer"
label = "hammer"
station = "crafting:hand"
inputs = [{ thing = "chip", count = 1 }]
outputs = [{ thing = "hammer" }]
work = 60

[[crafting.recipe]]
id = "struck"
label = "struck blade"
station = "crafting:hand"
inputs = [{ tag = "shard", count = 2 }]
outputs = [{ thing = "blade" }]
work = 60
requires = ["pounding"]

[[crafting.recipe]]
id = "plank"
label = "plank"
station = "crafting:bench"
inputs = [{ thing = "core:wood", count = 1 }]
outputs = [{ thing = "blade" }]
work = 90
"##;

fn kit_mods(name: &str) -> PathBuf {
    common::test_mods(name, &["core", "crafting"], &[("kit", &[("defs/kit.toml", DEFS)])])
}

/// One colonist, a finished station beside them, and its entity.
fn world(dir: &Path, station: &str) -> (Sim, Entity, Entity) {
    let mut s = Sim::new(dir, 3).unwrap_or_else(|e| panic!("mods load: {e}"));
    s.step();
    let founder = s.world.colonists().next().expect("a founder");
    for e in s.world.pawns.clone() {
        if e != founder {
            let _ = s.world.ecs.despawn(e);
        }
    }
    s.world.pawns.retain(|&e| e == founder);
    let at = free_cell(&s, s.world.pawn_pos(founder).unwrap());
    let def = s.world.defs.thing_id(station).unwrap();
    let wood = s.world.defs.thing_id("core:wood").unwrap();
    let stuff = s.world.defs.thing(def).build.as_ref().unwrap().stuff.is_some().then_some(wood);
    let e = s.world.spawn_fixture_of(def, at, false, stuff).expect("placed");
    rim_sim::ai::complete_building(&mut s.world, e);
    (s, founder, e)
}

/// An open cell near `home` with open cells all round, for a station and
/// the spot in front of it.
fn free_cell(s: &Sim, home: IVec) -> IVec {
    let open =
        |p: IVec| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none();
    (2..10)
        .flat_map(|d| [home.offset(d, 0), home.offset(-d, 0), home.offset(0, d), home.offset(0, -d)])
        .find(|&p| open(p) && [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().all(|&(dx, dy)| open(p.offset(dx, dy))))
        .expect("room for a station")
}

fn put(s: &mut Sim, id: &str, n: u32, near: IVec) {
    let d = s.world.defs.thing_id(id).unwrap();
    s.world.place_item(d, near, n);
}

fn table(pairs: &[(&str, Data)]) -> Data {
    Data::Table(pairs.iter().map(|(k, v)| (Key::Str((*k).into()), v.clone())).collect())
}

fn id(e: Entity) -> Data {
    Data::Int(e.to_bits().get() as i64)
}

fn send(s: &mut Sim, name: &str, pairs: &[(&str, Data)]) {
    s.push(Command::ModEvent { name: name.into(), data: Some(table(pairs)) });
}

fn add_bill(s: &mut Sim, site: Entity, recipe: &str) {
    send(s, "crafting:add_bill", &[("site", id(site)), ("recipe", Data::Str(recipe.into()))]);
    s.step();
}

/// The site's bills as script data, in order.
fn bills(s: &Sim, site: Entity) -> Vec<Data> {
    let Some(Data::Table(state)) = s.world.data.get("crafting:bills") else { return Vec::new() };
    let st = state.values().find(|st| field(st, "site") == Some(&id(site)));
    match st.and_then(|st| field(st, "bills")) {
        Some(Data::Table(b)) => b.values().cloned().collect(),
        _ => Vec::new(),
    }
}

fn field<'a>(d: &'a Data, k: &str) -> Option<&'a Data> {
    match d {
        Data::Table(t) => t.get(&Key::Str(k.into())),
        _ => None,
    }
}

fn first_bill(s: &Sim, site: Entity) -> Data {
    bills(s, site).first().cloned().expect("a bill")
}

fn set_bill(s: &mut Sim, site: Entity, changes: &[(&str, Data)]) {
    let bill = field(&first_bill(s, site), "id").cloned().unwrap();
    let mut pairs = vec![("site", id(site)), ("bill", bill)];
    pairs.extend(changes.iter().cloned());
    send(s, "crafting:set_bill", &pairs);
    s.step();
}

fn blades(s: &Sim) -> Vec<(Entity, u32, Option<rim_sim::defs::DefId>, i32)> {
    let blade = s.world.defs.thing_id("kit:blade").unwrap();
    s.world
        .ecs
        .query::<(Entity, &Thing, Option<&MadeOf>)>()
        .iter()
        .filter(|(_, t, _)| t.def == blade)
        .map(|(e, t, m)| (e, t.count, m.map(|m| m.0), t.hp))
        .collect()
}

fn count(s: &Sim, thing: &str) -> u32 {
    let d = s.world.defs.thing_id(thing).unwrap();
    s.world.ecs.query::<&Thing>().iter().filter(|t| t.def == d).map(|t| t.count).sum()
}

/// Step until the bills hook posts an order on `site`, and return the
/// hook's phase: it runs again on the step after which `tick % 60` is this.
fn posted(s: &mut Sim, site: Entity) -> u64 {
    assert!(run_until(s, 200, |s| s.world.ecs.get::<&Order>(site).is_ok()), "an order is posted");
    s.world.tick % 60
}

/// Step until the next step is the one the hook runs on.
fn before_hook(s: &mut Sim, phase: u64) {
    while (s.world.tick + 1) % 60 != phase {
        s.step();
    }
}

/// Step once, on a step the hook doesn't run on.
fn step_off_hook(s: &mut Sim, phase: u64) {
    if (s.world.tick + 1) % 60 == phase {
        s.step();
    }
    s.step();
}

fn delivered(s: &Sim, site: Entity) -> bool {
    s.world.ecs.get::<&Order>(site).is_ok_and(|o| o.missing().is_none())
}

fn steps(s: &mut Sim, n: u32) {
    for _ in 0..n {
        s.step();
    }
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
fn a_bill_runs_to_its_count_and_is_done() {
    let (mut s, founder, spot) = world(&kit_mods("craft-count"), "crafting:spot");
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "kit:chip", 5, home);
    add_bill(&mut s, spot, "kit:blade");
    set_bill(&mut s, spot, &[("target", Data::Int(2))]);
    assert!(run_until(&mut s, 12_000, |s| count(s, "kit:blade") == 2), "two blades are made");
    steps(&mut s, 3_000);
    assert_eq!(count(&s, "kit:blade"), 2, "and no more");
    assert_eq!(count(&s, "kit:chip"), 1);
    assert!(bills(&s, spot).is_empty(), "a finished bill is gone");
    let chip = s.world.defs.thing_id("kit:chip").unwrap();
    let (_, _, made_of, hp) = blades(&s)[0];
    assert_eq!((made_of, hp), (Some(chip), 40), "made of chip, at chip's hp");
    let crafting = s.world.defs.lookup("skill", "crafting:crafting").expect("the plugin's skill");
    let pawn = s.world.ecs.get::<&rim_sim::world::Pawn>(founder).unwrap();
    assert!(pawn.skills.iter().any(|&(k, xp)| k == crafting && xp > 0), "and the crafter learned by it");
}

#[test]
fn until_you_have_n_stops_and_starts_again() {
    let (mut s, founder, spot) = world(&kit_mods("craft-until"), "crafting:spot");
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "kit:chip", 12, home);
    add_bill(&mut s, spot, "kit:blade");
    set_bill(&mut s, spot, &[("mode", Data::Str("until".into())), ("target", Data::Int(2))]);
    assert!(run_until(&mut s, 12_000, |s| count(s, "kit:blade") == 2));
    steps(&mut s, 3_000);
    assert_eq!(count(&s, "kit:blade"), 2, "it stops at two");
    assert_eq!(field(&first_bill(&s, spot), "why"), Some(&Data::Str("have 2".into())));
    let (e, n, _, _) = blades(&s)[0];
    if n == 1 {
        s.world.despawn_thing(e);
    } else {
        s.world.ecs.get::<&mut Thing>(e).unwrap().count -= 1;
    }
    assert!(run_until(&mut s, 12_000, |s| count(s, "kit:blade") == 2), "one used, one made");
    assert_eq!(bills(&s, spot).len(), 1, "the bill stays");
}

#[test]
fn forever_runs_until_the_inputs_run_out() {
    let (mut s, founder, spot) = world(&kit_mods("craft-forever"), "crafting:spot");
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "kit:chip", 6, home);
    add_bill(&mut s, spot, "kit:blade");
    set_bill(&mut s, spot, &[("mode", Data::Str("forever".into()))]);
    assert!(run_until(&mut s, 20_000, |s| count(s, "kit:blade") == 3), "every chip made into blades");
    steps(&mut s, 200);
    let b = first_bill(&s, spot);
    assert_eq!(field(&b, "done"), Some(&Data::Int(3)));
    assert_eq!(field(&b, "why"), Some(&Data::Str("no shard".into())), "and it waits for more");
}

#[test]
fn a_stalled_bill_says_why() {
    let (mut s, founder, spot) = world(&kit_mods("craft-why"), "crafting:spot");
    add_bill(&mut s, spot, "kit:blade");
    steps(&mut s, 200);
    assert_eq!(field(&first_bill(&s, spot), "why"), Some(&Data::Str("no shard".into())));
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "kit:chip", 1, home);
    steps(&mut s, 200);
    assert_eq!(field(&first_bill(&s, spot), "why"), Some(&Data::Str("1 of 2 shard".into())));
    assert!(s.world.ecs.get::<&Order>(spot).is_err(), "nothing posted that can't be supplied");
}

#[test]
fn a_paused_bill_waits() {
    let (mut s, founder, spot) = world(&kit_mods("craft-pause"), "crafting:spot");
    add_bill(&mut s, spot, "kit:blade");
    set_bill(&mut s, spot, &[("suspended", Data::Bool(true))]);
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "kit:chip", 2, home);
    steps(&mut s, 500);
    assert!(s.world.ecs.get::<&Order>(spot).is_err());
    set_bill(&mut s, spot, &[("suspended", Data::Bool(false))]);
    assert!(run_until(&mut s, 12_000, |s| count(s, "kit:blade") == 1));
}

#[test]
fn removing_a_bill_puts_back_what_was_brought() {
    let (mut s, founder, spot) = world(&kit_mods("craft-remove"), "crafting:spot");
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "kit:chip", 2, home);
    add_bill(&mut s, spot, "kit:blade");
    assert!(run_until(&mut s, 6_000, |s| s.world.ecs.get::<&Order>(spot).is_ok_and(|o| o.needs[0].have() == 2)));
    let bill = field(&first_bill(&s, spot), "id").cloned().unwrap();
    send(&mut s, "crafting:remove_bill", &[("site", id(spot)), ("bill", bill)]);
    s.step();
    assert!(s.world.ecs.get::<&Order>(spot).is_err(), "its order is taken down");
    assert_eq!(count(&s, "kit:chip"), 2, "and the chips are back on the ground");
    steps(&mut s, 2_000);
    assert_eq!(count(&s, "kit:blade"), 0);
}

#[test]
fn a_workbench_is_faster_and_makes_bench_recipes() {
    let dir = kit_mods("craft-bench");
    let (mut s, founder, bench) = world(&dir, "crafting:workbench");
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "core:wood", 1, home);
    add_bill(&mut s, bench, "kit:plank");
    assert!(run_until(&mut s, 200, |s| s.world.ecs.get::<&Order>(bench).is_ok()));
    assert_eq!(s.world.ecs.get::<&Order>(bench).unwrap().work, 60, "90 work at 1.5");
    assert!(run_until(&mut s, 12_000, |s| count(s, "kit:blade") == 1));

    let (mut s, _, spot) = world(&dir, "crafting:spot");
    add_bill(&mut s, spot, "kit:plank");
    assert!(bills(&s, spot).is_empty(), "a spot is no bench");
}

#[test]
fn a_station_torn_down_takes_its_bills() {
    let (mut s, founder, spot) = world(&kit_mods("craft-lost"), "crafting:spot");
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "kit:chip", 2, home);
    add_bill(&mut s, spot, "kit:slow");
    let phase = posted(&mut s, spot);
    assert!(run_until(&mut s, 6_000, |s| delivered(s, spot)));
    s.world.despawn_thing(spot);
    // Heard as order_lost, not found later by the hook.
    step_off_hook(&mut s, phase);
    assert_eq!(s.world.data.get("crafting:bills"), Some(&Data::Table(Default::default())));
    assert_eq!(count(&s, "kit:chip"), 2, "what was brought is back on the ground");
}

/// Hooks run before events in a tick. An order that finishes on the hook's
/// tick is gone before its order_done is heard; the hook must not take the
/// station for idle and post again.
#[test]
fn an_order_finishing_on_the_hook_tick_is_made_once() {
    let (mut s, founder, spot) = world(&kit_mods("craft-race"), "crafting:spot");
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "kit:chip", 4, home);
    add_bill(&mut s, spot, "kit:slow");
    let phase = posted(&mut s, spot);
    assert!(run_until(&mut s, 6_000, |s| delivered(s, spot)));
    before_hook(&mut s, phase);
    rim_sim::ai::finish_order(&mut s.world, spot);
    s.step();
    steps(&mut s, 400);
    assert_eq!(count(&s, "kit:blade"), 1, "one blade");
    assert!(s.world.ecs.get::<&Order>(spot).is_err(), "and no second order for a finished bill");
    assert!(bills(&s, spot).is_empty());
    assert_eq!(count(&s, "kit:chip"), 2, "the other two chips untouched");
}

/// Removed on the tick it finished: too late to take down, so it's made.
#[test]
fn a_bill_removed_as_it_finishes_is_still_made() {
    let (mut s, founder, spot) = world(&kit_mods("craft-remove-race"), "crafting:spot");
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "kit:chip", 2, home);
    add_bill(&mut s, spot, "kit:slow");
    posted(&mut s, spot);
    assert!(run_until(&mut s, 6_000, |s| delivered(s, spot)));
    let bill = field(&first_bill(&s, spot), "id").cloned().unwrap();
    rim_sim::ai::finish_order(&mut s.world, spot);
    send(&mut s, "crafting:remove_bill", &[("site", id(spot)), ("bill", bill)]);
    s.step();
    assert_eq!(count(&s, "kit:blade"), 1, "the chips went in, so the blade comes out");
}

/// A bill for a tool nobody has doesn't hold the station: the bill below,
/// which makes the tool, runs first, and then the one above.
#[test]
fn a_bill_waiting_on_a_tool_lets_the_one_that_makes_it_run() {
    let (mut s, founder, spot) = world(&kit_mods("craft-tool-order"), "crafting:spot");
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "kit:chip", 3, home);
    add_bill(&mut s, spot, "kit:struck");
    add_bill(&mut s, spot, "kit:hammer");
    steps(&mut s, 200);
    assert_eq!(field(&first_bill(&s, spot), "why"), Some(&Data::Str("needs a pounding tool".into())));
    assert!(run_until(&mut s, 12_000, |s| count(s, "kit:blade") == 1), "the hammer, then the blade");
    assert_eq!(count(&s, "kit:hammer"), 1);
}

/// A recipe the engine refuses (here, under a work type that doesn't exist)
/// is paused, with the engine's reason on one line, not a stack trace in
/// the save.
#[test]
fn a_refused_recipe_is_paused_with_the_reason() {
    let (mut s, founder, spot) = world(&kit_mods("craft-refused"), "crafting:spot");
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "kit:chip", 1, home);
    add_bill(&mut s, spot, "kit:odd");
    steps(&mut s, 200);
    let b = first_bill(&s, spot);
    assert_eq!(field(&b, "suspended"), Some(&Data::Bool(true)));
    let Some(Data::Str(why)) = field(&b, "why") else { panic!("a reason: {b:?}") };
    assert!(why.contains("nope") && !why.contains('\n') && !why.starts_with("runtime error"), "{why}");
}

#[test]
fn bills_run_top_first_and_move() {
    let (mut s, founder, spot) = world(&kit_mods("craft-move"), "crafting:spot");
    add_bill(&mut s, spot, "kit:slow");
    add_bill(&mut s, spot, "kit:blade");
    let ids = |s: &Sim| bills(s, spot).iter().map(|b| field(b, "id").cloned().unwrap()).collect::<Vec<_>>();
    let before = ids(&s);
    send(&mut s, "crafting:move_bill", &[("site", id(spot)), ("bill", before[1].clone()), ("by", Data::Int(-1))]);
    s.step();
    assert_eq!(ids(&s), vec![before[1].clone(), before[0].clone()]);
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "kit:chip", 2, home);
    posted(&mut s, spot);
    assert_eq!(s.world.ecs.get::<&Order>(spot).unwrap().label, "blade", "the top bill is made first");
}

/// Bills are script data: a save in the middle of one restores to the same
/// world, and both play on the same.
#[test]
fn a_bill_survives_a_save() {
    let dir = kit_mods("craft-save");
    let (mut s, founder, spot) = world(&dir, "crafting:spot");
    let home = s.world.pawn_pos(founder).unwrap();
    put(&mut s, "kit:chip", 8, home);
    add_bill(&mut s, spot, "kit:blade");
    set_bill(&mut s, spot, &[("mode", Data::Str("until".into())), ("target", Data::Int(3))]);
    posted(&mut s, spot);
    let snap = Snapshot::capture(&s);
    let mut back = snap.restore(&dir, &|_| true).unwrap_or_else(|e| panic!("restores: {e}"));
    assert_eq!(Snapshot::capture(&back).hash(), snap.hash());
    steps(&mut s, 6_000);
    steps(&mut back, 6_000);
    assert_eq!(count(&s, "kit:blade"), 3);
    assert_eq!(Snapshot::capture(&back).hash(), Snapshot::capture(&s).hash(), "the same game from here");
}

/// A crafting spot is free: it's marked and built with nothing brought.
#[test]
fn a_crafting_spot_costs_nothing() {
    let mut s = Sim::new(&kit_mods("craft-free"), 3).unwrap();
    s.step();
    let founder = s.world.colonists().next().unwrap();
    let at = free_cell(&s, s.world.pawn_pos(founder).unwrap());
    let spot = s.world.defs.thing_id("crafting:spot").unwrap();
    s.push(Command::Build { thing: spot, stuff: None, a: at, b: at });
    let built = |s: &Sim| s.world.map.fixture_at(at).is_some_and(|e| s.world.ecs.get::<&Blueprint>(e).is_err());
    assert!(run_until(&mut s, 4_000, built), "the spot is marked");
}

/// Every TOML sample in docs/modding/crafting.md loads, in one mod beside
/// core and crafting, and its recipe is made where the guide says.
#[test]
fn guide_samples_load() {
    let guide =
        std::fs::read_to_string(common::mods().join("../docs/modding/crafting.md")).unwrap().replace("\r\n", "\n");
    let samples: Vec<&str> = guide.split("```toml\n").skip(1).map(|b| b.split("```").next().unwrap()).collect();
    assert_eq!(samples.len(), 2);
    let dir = common::test_mods(
        "craft-guide",
        &["core", "crafting"],
        &[("guide", &[("defs/guide.toml", &samples.join("\n"))])],
    );
    let mut s = Sim::new(&dir, 3).unwrap_or_else(|e| panic!("the guide's samples don't load: {e}"));
    s.step();
    let founder = s.world.colonists().next().unwrap();
    let home = s.world.pawn_pos(founder).unwrap();
    let anvil = s.world.defs.thing_id("guide:anvil").unwrap();
    let e = s.world.spawn_fixture_of(anvil, free_cell(&s, home), false, None).unwrap();
    rim_sim::ai::complete_building(&mut s.world, e);
    add_bill(&mut s, e, "guide:knife");
    assert!(bills(&s, e).is_empty(), "the anvil does smithing, not handwork");
    put(&mut s, "guide:shard", 1, home);
    put(&mut s, "core:wood", 1, home);
    let spot = s.world.defs.thing_id("crafting:spot").unwrap();
    let e = s.world.spawn_fixture_of(spot, free_cell(&s, home), false, None).unwrap();
    rim_sim::ai::complete_building(&mut s.world, e);
    add_bill(&mut s, e, "guide:knife");
    assert!(run_until(&mut s, 12_000, |s| count(s, "guide:knife") == 1), "a knife");
}

#[test]
fn free_with_a_cost_is_refused() {
    let defs = r##"
[[thing]]
id = "odd"
label = "odd"
color = "#000000"
category = "building"
look.layers = [{ draw = "fill" }]
build = { work = 10, free = true, cost = [{ thing = "core:wood", count = 1 }] }
"##;
    let dir = common::test_mods("craft-free-cost", &["core"], &[("odd", &[("defs/odd.toml", defs)])]);
    let err = Sim::new(&dir, 1).err().expect("refused");
    assert!(err.contains("`free` and has a cost"), "{err}");
    let costless = defs.replace(", free = true, cost = [{ thing = \"core:wood\", count = 1 }]", "");
    let dir = common::test_mods("craft-no-cost", &["core"], &[("odd", &[("defs/odd.toml", &costless)])]);
    let err = Sim::new(&dir, 1).err().expect("refused");
    assert!(err.contains("needs `cost`, `stuff`, or `free = true`"), "{err}");
}

#[test]
fn a_recipe_naming_a_missing_thing_is_an_error() {
    let defs = r##"
[[crafting.recipe]]
id = "nope"
label = "nope"
station = "crafting:hand"
inputs = [{ thing = "nope", count = 1 }]
outputs = [{ thing = "core:wood" }]
work = 10
"##;
    let dir = common::test_mods("craft-bad-ref", &["core", "crafting"], &[("bad", &[("defs/bad.toml", defs)])]);
    let err = Sim::new(&dir, 1).err().expect("the mod fails to load");
    assert!(err.contains("recipe 'bad:nope' input 1: no thing 'bad:nope'"), "{err}");
}
