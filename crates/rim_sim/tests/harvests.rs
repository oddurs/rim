//! Several harvests on one thing: an oak gathered for what regrows, then
//! chopped for what doesn't.

mod common;

use rim_sim::defs::DefId;
use rim_sim::hecs::Entity;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{Regrow, Thing};
use rim_sim::{Command, IVec, Sim};
use std::path::Path;

/// Core, plus a mod that adds a gather designation, appends a gathering
/// harvest to core's oak and speeds up its chop with an edit. Both patches
/// meet `harvest` as a single table.
const ORCHARD: &str = r##"
[[designation]]
id = "gather"
label = "Gather"
color = "#c8a060"
work_type = "core:harvest"

[[patch]]
target = "thing/core:tree_oak"
append = { harvest = [{ designation = "orchard:gather", work = 40, destroy = false, regrow_days = 1.0,
                        yields = [{ thing = "core:wood", count = 2 }] }] }

[[patch]]
target = "thing/core:tree_oak"
[[patch.edit]]
list = "harvest"
match = { designation = "chop" }
set = { work = 60 }
"##;

fn orchard(name: &str) -> std::path::PathBuf {
    common::test_mods(name, &["core"], &[("orchard", &[("defs/orchard.toml", ORCHARD)])])
}

/// A world with one colonist, so nobody else takes the work.
fn alone(dir: &Path) -> (Sim, Entity) {
    let mut s = Sim::new(dir, 21).unwrap_or_else(|e| panic!("mods load: {e}"));
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

fn designation(s: &Sim, id: &str) -> DefId {
    s.world.defs.lookup("designation", id).unwrap_or_else(|| panic!("designation {id}"))
}

/// The oak nearest the colony that the founder can reach.
fn an_oak(s: &Sim) -> (Entity, IVec) {
    let oak = s.world.defs.thing_id("tree_oak").unwrap();
    let c = s.world.colony_center().unwrap();
    s.world
        .ecs
        .query::<(Entity, &Thing)>()
        .iter()
        .filter(|(_, t)| t.def == oak && s.world.map.can_reach(c, rim_sim::path::Goal::Touch(t.pos)))
        .map(|(e, t)| (e, t.pos))
        .min_by_key(|(e, p)| (p.octile(c), e.id()))
        .expect("an oak in reach")
}

fn wood_near(s: &Sim, at: IVec) -> u32 {
    let wood = s.world.defs.thing_id("wood").unwrap();
    s.world.ecs.query::<&Thing>().iter().filter(|t| t.def == wood && t.pos.chebyshev(at) <= 3).map(|t| t.count).sum()
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
fn harvest_is_one_table_or_a_list() {
    let s = Sim::with_mods(&common::mods(), 1, &|m| m == "core").unwrap();
    assert_eq!(s.world.defs.thing(s.world.defs.thing_id("tree_oak").unwrap()).harvest.len(), 1, "core is unchanged");

    let (s, _) = alone(&orchard("harvest-list"));
    let oak = s.world.defs.thing(s.world.defs.thing_id("tree_oak").unwrap());
    let chop = oak.harvest_for(designation(&s, "chop")).expect("chop kept");
    let gather = oak.harvest_for(designation(&s, "orchard:gather")).expect("gather appended");
    assert_eq!((chop.index, chop.work, chop.destroy), (0, 60, true), "the edit reached a single-table harvest");
    assert_eq!((gather.index, gather.destroy), (1, false));
}

#[test]
fn two_harvests_with_one_designation_fail_the_load() {
    let twice = r##"
[[patch]]
target = "thing/core:tree_oak"
append = { harvest = [{ designation = "chop", work = 10 }] }
"##;
    let dir = common::test_mods("harvest-twice", &["core"], &[("twice", &[("defs/twice.toml", twice)])]);
    let err = Sim::new(&dir, 1).err().expect("the load fails");
    assert!(err.contains("two harvests use the designation 'chop'"), "{err}");
}

#[test]
fn a_tree_is_gathered_while_it_stands_then_chopped() {
    let (mut s, _) = alone(&orchard("harvest-gather-chop"));
    let (tree, at) = an_oak(&s);
    let before = wood_near(&s, at);

    s.push(Command::Designate { designation: designation(&s, "orchard:gather"), a: at, b: at });
    assert!(run_until(&mut s, 3_000, |s| wood_near(s, at) >= before + 2), "gathered");
    assert!(s.world.thing(tree).is_some(), "gathering leaves the tree standing");
    let gather = Some(designation(&s, "orchard:gather"));
    let r: Regrow = s.world.ecs.get::<&Regrow>(tree).map(|r| (*r).clone()).expect("regrowing");
    assert!(r.growing(gather) && !r.growing(None), "only the gathered harvest regrows: {r:?}");

    // While the branches regrow, a chop is still available.
    s.push(Command::Designate { designation: designation(&s, "chop"), a: at, b: at });
    assert!(run_until(&mut s, 3_000, |s| s.world.thing(tree).is_none()), "chopped while regrowing");
    assert!(wood_near(&s, at) >= before + 2 + 20, "the chop's wood");
}

#[test]
fn regrowth_is_per_harvest_and_survives_a_save() {
    let (mut s, _) = alone(&orchard("harvest-save"));
    let (tree, at) = an_oak(&s);
    s.push(Command::Designate { designation: designation(&s, "orchard:gather"), a: at, b: at });
    assert!(run_until(&mut s, 3_000, |s| s.world.ecs.get::<&Regrow>(tree).is_ok()), "gathered");

    let snap = Snapshot::capture(&s);
    let back = snap.restore(&orchard("harvest-save"), &|_| true).unwrap_or_else(|e| panic!("restores: {e}"));
    let gather = Some(designation(&s, "orchard:gather"));
    let r: Regrow = back.world.ecs.get::<&Regrow>(tree).map(|r| (*r).clone()).expect("still regrowing after a load");
    assert!(r.growing(gather) && !r.growing(None));
    assert_eq!(Snapshot::capture(&back).hash(), snap.hash());

    // Ready again after a day.
    let ready_at = r.ready_at;
    assert!(run_until(&mut s, 25_000, |s| s.world.harvest_ready(tree, gather)), "regrows");
    assert!(s.world.tick >= ready_at && s.world.ecs.get::<&Regrow>(tree).is_err());
}

/// A thing with one harvest regrowing saves byte for byte as it did when
/// `Regrow` was only `ready_at`, so older saves and their hashes still hold.
#[test]
fn one_regrowing_harvest_saves_as_it_always_did() {
    #[derive(serde::Serialize)]
    struct Before {
        ready_at: u64,
    }
    let now = rmp_serde::to_vec_named(&Regrow::new(None, 4_321)).unwrap();
    assert_eq!(now, rmp_serde::to_vec_named(&Before { ready_at: 4_321 }).unwrap());
    let old: Regrow = rmp_serde::from_slice(&now).unwrap();
    assert!(old.growing(None) && !old.growing(Some(3)));
}

#[test]
fn several_harvests_can_regrow_at_once() {
    let mut r = Regrow::new(None, 100);
    r.also.push((Some(7), 50));
    assert!(r.growing(None) && r.growing(Some(7)) && !r.growing(Some(8)));
    assert!(r.ripen(60), "one left");
    assert!(r.growing(None) && !r.growing(Some(7)));
    assert!(!r.ripen(100), "none left");
}

/// A regrowing harvest is named by its designation, so a load under
/// another mod list (other designation ids) still knows which one it is.
#[test]
fn regrowth_follows_its_designation_across_a_change_of_mods() {
    let (mut s, _) = alone(&orchard("harvest-remap"));
    let (tree, at) = an_oak(&s);
    s.push(Command::Designate { designation: designation(&s, "orchard:gather"), a: at, b: at });
    assert!(run_until(&mut s, 3_000, |s| s.world.ecs.get::<&Regrow>(tree).is_ok()), "gathered");
    let snap = Snapshot::capture(&s);

    // Another mod adds designations ahead of orchard's, so every id after
    // core's moves.
    let extra = r##"
[[designation]]
id = "aardvark"
label = "Aardvark"
color = "#808080"
work_type = "core:harvest"
"##;
    let moved = common::test_mods(
        "harvest-remap-b",
        &["core"],
        &[("aaa", &[("defs/aaa.toml", extra)]), ("orchard", &[("defs/orchard.toml", ORCHARD)])],
    );
    let (back, _) = snap.restore_noting(&moved, &|_| true).unwrap_or_else(|e| panic!("restores: {e}"));
    let gather = back.world.defs.lookup("designation", "orchard:gather").unwrap();
    assert_ne!(Some(gather), Some(designation(&s, "orchard:gather")), "the id moved");
    let r: Regrow = back.world.ecs.get::<&Regrow>(tree).map(|r| (*r).clone()).expect("still regrowing");
    assert!(r.growing(Some(gather)) && !r.growing(None), "{r:?}");
}

/// Right-click on a thing with several harvests takes the gentlest one
/// that's ready, or the one it's designated for: never a chop the player
/// didn't ask for.
#[test]
fn a_click_gathers_rather_than_fells() {
    let (mut s, founder) = alone(&orchard("harvest-click"));
    let (tree, at) = an_oak(&s);
    let order = |s: &Sim| rim_sim::order::resolve(&s.world, founder, at, None).map(|o| o.label);
    assert_eq!(order(&s).as_deref(), Some("Gather oak tree"));

    s.push(Command::Designate { designation: designation(&s, "chop"), a: at, b: at });
    s.step();
    assert_eq!(order(&s).as_deref(), Some("Chop oak tree"), "the designated harvest");

    // Gathered and marked to gather again: no harvest to offer until it
    // regrows, and the chop it isn't marked for is not offered instead.
    s.push(Command::Designate { designation: designation(&s, "orchard:gather"), a: at, b: at });
    assert!(run_until(&mut s, 3_000, |s| s.world.ecs.get::<&Regrow>(tree).is_ok()), "gathered");
    s.push(Command::Designate { designation: designation(&s, "orchard:gather"), a: at, b: at });
    s.step();
    assert_eq!(order(&s).as_deref(), Some("Go here"), "no harvest, and above all no chop");
}

/// `set` on a harvest that another patch made a list of two can't say which
/// it means: reported and skipped, rather than replacing both.
#[test]
fn set_on_several_harvests_is_reported_not_applied() {
    let append = r##"
[[patch]]
target = "thing/core:berry_bush"
append = { harvest = [{ designation = "chop", work = 90, yields = [{ thing = "core:wood", count = 1 }] }] }
"##;
    let set = r##"
[[patch]]
target = "thing/core:berry_bush"
set = { harvest = { regrow_days = 9.0 } }
"##;
    let dir = common::test_mods(
        "harvest-set",
        &["core"],
        &[("a_append", &[("defs/a.toml", append)]), ("b_set", &[("defs/b.toml", set)])],
    );
    let s = Sim::new(&dir, 1).unwrap_or_else(|e| panic!("loads: {e}"));
    let bush = s.world.defs.thing(s.world.defs.thing_id("berry_bush").unwrap());
    assert_eq!(bush.harvest.len(), 2);
    assert_eq!(bush.harvest[0].regrow_days, 2.0, "unchanged");
    assert!(s.warnings.iter().any(|w| w.contains("can't say which")), "{:?}", s.warnings);
}

/// A designated harvest that's growing back says so, for the inspector.
#[test]
fn a_regrowing_designation_says_why() {
    let (mut s, _) = alone(&orchard("harvest-why"));
    let (tree, at) = an_oak(&s);
    let gather = designation(&s, "orchard:gather");
    assert_eq!(rim_sim::ai::work_blocked(&s.world, tree), None, "not designated");
    s.push(Command::Designate { designation: gather, a: at, b: at });
    assert!(run_until(&mut s, 3_000, |s| s.world.ecs.get::<&Regrow>(tree).is_ok()), "gathered");
    s.push(Command::Designate { designation: gather, a: at, b: at });
    s.step();
    assert_eq!(rim_sim::ai::work_blocked(&s.world, tree).as_deref(), Some("Growing back."));
}
