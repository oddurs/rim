//! Tools in hand (DESIGN.md §4e): work that needs a tool waits for one, a
//! pawn fetches it, it sets the pace, and it wears out.

mod common;

use rim_sim::hecs::Entity;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{Held, Pawn, Thing, Work};
use rim_sim::{Command, IVec, Sim};
use std::path::{Path, PathBuf};

/// An axe that chops twice as fast and breaks on its third tree, a maul
/// for rock, and core's chop and mine gated behind them.
const KIT: &str = r##"
[[thing]]
id = "axe"
label = "axe"
color = "#8a6a4a"
category = "item"
look.layers = [{ draw = "fill" }]
hp = 100
tool = { tags = ["chopping", "cutting"], speed = 2.0, wear = 40 }

[[thing]]
id = "maul"
label = "maul"
color = "#6a6a6a"
category = "item"
look.layers = [{ draw = "fill" }]
tool = { tags = ["pounding"] }

[[patch]]
target = "thing/core:tree_oak"
set = { harvest = { requires = ["chopping"] } }

[[patch]]
target = "thing/core:granite"
set = { harvest = { requires = ["pounding"] } }
"##;

fn kit(name: &str) -> PathBuf {
    common::test_mods(name, &["core"], &[("kit", &[("defs/kit.toml", KIT)])])
}

/// One colonist, so nobody else takes the work.
fn alone(dir: &Path) -> (Sim, Entity) {
    let mut s = Sim::new(dir, 3).unwrap_or_else(|e| panic!("mods load: {e}"));
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

/// The `n` oaks nearest the colony that it can reach.
fn oaks(s: &Sim, n: usize) -> Vec<(Entity, IVec)> {
    let oak = s.world.defs.thing_id("tree_oak").unwrap();
    let c = s.world.colony_center().unwrap();
    let mut all: Vec<(Entity, IVec)> = s
        .world
        .ecs
        .query::<(Entity, &Thing)>()
        .iter()
        .filter(|(_, t)| t.def == oak && s.world.map.can_reach(c, rim_sim::path::Goal::Touch(t.pos)))
        .map(|(e, t)| (e, t.pos))
        .collect();
    all.sort_by_key(|(e, p)| (p.octile(c), e.id()));
    all.truncate(n);
    all
}

fn chop(s: &mut Sim, at: IVec) {
    let d = s.world.defs.lookup("designation", "core:chop").unwrap();
    s.push(Command::Designate { designation: d, a: at, b: at });
}

fn hand(s: &Sim, e: Entity) -> Option<Entity> {
    s.world.ecs.get::<&Pawn>(e).unwrap().hand
}

/// Put a tool on the map beside the colonist; its entity.
fn place(s: &mut Sim, id: &str, near: Entity) -> Entity {
    let def = s.world.defs.thing_id(id).unwrap();
    let at = s.world.pawn_pos(near).unwrap();
    s.world.place_item(def, at, 1);
    s.world.ecs.query::<(Entity, &Thing)>().iter().find(|(_, t)| t.def == def).map(|(e, _)| e).expect("placed")
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
fn without_a_tool_the_tree_stands_and_says_why() {
    let (mut s, _) = alone(&kit("tools-none"));
    let (tree, at) = oaks(&s, 1)[0];
    chop(&mut s, at);
    assert!(!run_until(&mut s, 3_000, |s| s.world.thing(tree).is_none()), "nobody can chop it");
    assert_eq!(rim_sim::ai::work_blocked(&s.world, tree).as_deref(), Some("Needs a chopping tool."));
}

#[test]
fn a_tool_is_fetched_and_sets_the_pace() {
    let (mut s, founder) = alone(&kit("tools-fetch"));
    let (tree, at) = oaks(&s, 1)[0];
    chop(&mut s, at);
    s.step();
    // Made after the work was marked: nothing needs reloading.
    let axe = place(&mut s, "axe", founder);
    assert!(run_until(&mut s, 3_000, |s| s.world.ecs.get::<&Work>(tree).is_ok()), "work starts");
    assert_eq!(hand(&s, founder), Some(axe), "holding the axe");
    assert!(s.world.ecs.get::<&Held>(axe).is_ok() && s.world.map.item_at(s.world.thing(axe).unwrap().pos) != Some(axe));
    let total = s.world.ecs.get::<&Work>(tree).unwrap().total;
    assert_eq!(total, 140, "280 work at speed 2");
    assert!(run_until(&mut s, 3_000, |s| s.world.thing(tree).is_none()), "felled");
}

#[test]
fn wear_breaks_a_tool() {
    let (mut s, founder) = alone(&kit("tools-wear"));
    let axe = place(&mut s, "axe", founder);
    for (i, (tree, at)) in oaks(&s, 3).into_iter().enumerate() {
        chop(&mut s, at);
        assert!(run_until(&mut s, 4_000, |s| s.world.thing(tree).is_none()), "tree {i} felled");
    }
    assert!(s.world.thing(axe).is_none() && hand(&s, founder).is_none(), "100 hp at 40 a tree: gone on the third");
    assert!(s.world.messages.iter().any(|m| m.text.ends_with("axe broke.")), "and the player is told");
}

#[test]
fn a_pawn_swaps_tools_for_the_job() {
    let (mut s, founder) = alone(&kit("tools-swap"));
    let axe = place(&mut s, "axe", founder);
    let (tree, at) = oaks(&s, 1)[0];
    chop(&mut s, at);
    assert!(run_until(&mut s, 4_000, |s| s.world.thing(tree).is_none()), "felled");
    assert_eq!(hand(&s, founder), Some(axe));

    // Granite needs pounding: the axe goes down where the maul was.
    let maul = place(&mut s, "maul", founder);
    let granite = s.world.defs.thing_id("granite").unwrap();
    let c = s.world.colony_center().unwrap();
    let rock = s
        .world
        .ecs
        .query::<(Entity, &Thing)>()
        .iter()
        .filter(|(_, t)| t.def == granite && s.world.map.can_reach(c, rim_sim::path::Goal::Touch(t.pos)))
        .map(|(e, t)| (e, t.pos))
        .min_by_key(|(e, p)| (p.octile(c), e.id()))
        .expect("granite in reach");
    let mine = s.world.defs.lookup("designation", "core:mine").unwrap();
    s.push(Command::Designate { designation: mine, a: rock.1, b: rock.1 });
    assert!(run_until(&mut s, 4_000, |s| hand(s, founder) == Some(maul)), "takes the maul");
    let put = s.world.thing(axe).expect("the axe is still around").pos;
    assert_eq!(s.world.map.item_at(put), Some(axe), "put down on the map");
    assert!(s.world.ecs.get::<&Held>(axe).is_err());
}

#[test]
fn a_held_tool_survives_a_save() {
    let dir = kit("tools-save");
    let (mut s, founder) = alone(&dir);
    let axe = place(&mut s, "axe", founder);
    let (_, at) = oaks(&s, 1)[0];
    chop(&mut s, at);
    assert!(run_until(&mut s, 3_000, |s| hand(s, founder) == Some(axe)), "holding it");
    let snap = Snapshot::capture(&s);
    let back = snap.restore(&dir, &|_| true).unwrap_or_else(|e| panic!("restores: {e}"));
    assert_eq!(hand(&back, founder), Some(axe));
    assert!(back.world.ecs.get::<&Held>(axe).is_ok() && back.world.tools.contains(&axe));
    assert_ne!(back.world.map.item_at(back.world.thing(axe).unwrap().pos), Some(axe), "not on the map");
    assert_eq!(Snapshot::capture(&back).hash(), snap.hash());
}

#[test]
fn a_tool_is_dropped_where_its_holder_dies() {
    let (mut s, founder) = alone(&kit("tools-death"));
    let axe = place(&mut s, "axe", founder);
    let (_, at) = oaks(&s, 1)[0];
    chop(&mut s, at);
    assert!(run_until(&mut s, 3_000, |s| hand(s, founder) == Some(axe)), "holding it");
    let pos = s.world.pawn_pos(founder).unwrap();
    s.world.ecs.get::<&mut Pawn>(founder).unwrap().dead = true;
    s.step();
    let t = s.world.thing(axe).expect("the axe outlives them");
    assert_eq!(s.world.map.item_at(t.pos), Some(axe));
    assert!(t.pos.chebyshev(pos) <= 1);
}

#[test]
fn core_alone_needs_no_tools() {
    let s = Sim::with_mods(&common::mods(), 1, &|m| m == "core").unwrap();
    assert!(s.world.defs.things.iter().all(|t| t.tool.is_none() && t.harvest.iter().all(|h| h.requires.is_empty())));
}

/// Put a tool on the map `n` cells from the colonist, on open ground.
fn place_away(s: &mut Sim, id: &str, near: Entity, n: i32) -> (Entity, IVec) {
    let def = s.world.defs.thing_id(id).unwrap();
    let from = s.world.pawn_pos(near).unwrap();
    let at = [(n, 0), (-n, 0), (0, n), (0, -n)]
        .into_iter()
        .map(|(dx, dy)| from.offset(dx, dy))
        .find(|&p| s.world.map.passable(p) && s.world.map.item_at(p).is_none() && s.world.map.fixture_at(p).is_none())
        .expect("open ground");
    s.world.place_item(def, at, 1);
    (s.world.map.item_at(at).expect("placed"), at)
}

#[test]
fn a_tool_is_fetched_from_where_it_lies() {
    let (mut s, founder) = alone(&kit("tools-walk"));
    let (axe, at) = place_away(&mut s, "axe", founder, 5);
    let (_, tree) = oaks(&s, 1)[0];
    chop(&mut s, tree);
    assert!(run_until(&mut s, 3_000, |s| hand(s, founder) == Some(axe)), "fetched");
    assert!(s.world.pawn_pos(founder).unwrap().chebyshev(at) <= 1, "by walking to it");
}

#[test]
fn two_colonists_share_one_axe_by_turns() {
    let (mut s, founder) = alone(&kit("tools-race"));
    let human = s.world.defs.creature_id("human").unwrap();
    let at = s.world.pawn_pos(founder).unwrap();
    let second = s.world.spawn_pawn(human, rim_sim::world::Faction::Player, at, None);
    let cols = [founder, second];
    let axe = place(&mut s, "axe", cols[0]);
    let trees = oaks(&s, 2);
    for &(_, at) in &trees {
        chop(&mut s, at);
    }
    let both = |s: &Sim| trees.iter().all(|(t, _)| s.world.thing(*t).is_none());
    for _ in 0..8_000 {
        s.step();
        let holders = cols.iter().filter(|&&c| hand(&s, c) == Some(axe)).count();
        assert!(holders <= 1, "one axe, one hand");
        if both(&s) {
            break;
        }
    }
    assert!(both(&s), "both felled, one after the other");
}

#[test]
fn a_colonist_who_leaves_takes_their_tool() {
    let (mut s, founder) = alone(&kit("tools-leave"));
    let axe = place(&mut s, "axe", founder);
    let (_, at) = oaks(&s, 1)[0];
    chop(&mut s, at);
    assert!(run_until(&mut s, 3_000, |s| hand(s, founder) == Some(axe)), "holding it");
    s.world.ecs.get::<&mut Pawn>(founder).unwrap().left = true;
    s.step();
    assert!(s.world.thing(axe).is_none() && !s.world.tools.contains(&axe));
}

/// A tool marked held that no hand holds (its holder didn't load) goes back
/// on the map rather than vanishing.
#[test]
fn a_held_tool_nobody_holds_is_on_the_map_after_a_load() {
    let dir = kit("tools-orphan");
    let (mut s, founder) = alone(&dir);
    let axe = place(&mut s, "axe", founder);
    let (_, at) = oaks(&s, 1)[0];
    chop(&mut s, at);
    assert!(run_until(&mut s, 3_000, |s| hand(s, founder) == Some(axe)), "holding it");
    s.world.ecs.get::<&mut Pawn>(founder).unwrap().hand = None;
    let back = Snapshot::capture(&s).restore(&dir, &|_| true).unwrap();
    let pos = back.world.thing(axe).expect("still there").pos;
    assert!(back.world.ecs.get::<&Held>(axe).is_err());
    assert_eq!(back.world.map.item_at(pos), Some(axe));
}

#[test]
fn a_tool_that_does_nothing_fails_the_load() {
    let bad = r##"
[[thing]]
id = "limp"
label = "limp stick"
color = "#806040"
category = "item"
look.layers = [{ draw = "fill" }]
tool = { tags = ["chopping"], speed = 0.0 }
"##;
    let dir = common::test_mods("tools-limp", &["core"], &[("limp", &[("defs/limp.toml", bad)])]);
    let err = Sim::new(&dir, 1).err().expect("refused");
    assert!(err.contains("tool speed must be above 0"), "{err}");
}

/// A click offers only what the pawn could do: no chop without an axe, and
/// with one lying about, a chop that fetches it first.
#[test]
fn a_click_offers_a_chop_only_with_an_axe_to_hand() {
    let (mut s, founder) = alone(&kit("tools-click"));
    let (_, at) = oaks(&s, 1)[0];
    let order = |s: &Sim| rim_sim::order::resolve(&s.world, founder, at, None);
    assert_ne!(order(&s).map(|o| o.label).as_deref(), Some("Chop oak tree"), "no axe, no chop");
    let (axe, _) = place_away(&mut s, "axe", founder, 4);
    let o = order(&s).expect("an order");
    assert_eq!(o.label, "Chop oak tree");
    assert!(matches!(o.job, rim_sim::world::Job::Harvest { tool: Some(t), .. } if t == axe), "fetching the axe first");
    assert!(o.reserve.contains(&axe), "and claiming it");
}
