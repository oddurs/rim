//! Containers (DESIGN.md §4f): things whose slots hold stacks, off the item
//! layer, sorted, reserved and counted like any store.

mod common;

use rim_sim::defs::DefId;
use rim_sim::filter::FilterEdit;
use rim_sim::hecs::Entity;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{Contained, Lot, Store, Thing};
use rim_sim::zone::StoreRef;
use rim_sim::{Command, IVec, Sim};
use std::path::PathBuf;

/// A test mod with a crate (4 slots, nothing bulky) and a box (2 slots,
/// anything).
const DEFS: &str = r##"
[[thing]]
id = "crate"
label = "crate"
color = "#b98a55"
category = "building"
blocks = true
hp = 100
store = { slots = 4, accepts = { not_tags = ["bulky"] } }

[[thing]]
id = "box"
label = "box"
color = "#8a6a45"
category = "building"
blocks = true
hp = 100
store = { slots = 2, stack_scale = 2, display = "items" }
"##;

fn mods(name: &str) -> PathBuf {
    common::test_mods(name, &["core", "crafting", "primitive"], &[("boxes", &[("defs/boxes.toml", DEFS)])])
}

/// A hauling colony (see common) on the test mods, and open ground (3 wide,
/// 5 deep from one above `site`) for containers a few cells from the
/// colonist.
fn colony(name: &str) -> (Sim, Entity, IVec, PathBuf) {
    let dir = mods(name);
    let mut sim = Sim::with_mods(&dir, 5, &|_| true).unwrap();
    let pawn = sim.world.colonists().next().unwrap();
    let defs = sim.world.defs.clone();
    for (w, d) in defs.work_types.iter().enumerate() {
        let level = if d.id == "core:haul" || d.id == "core:build" { 1 } else { 0 };
        sim.push(Command::SetPriority { pawn, work: w as DefId, level });
    }
    let c = sim.world.pawn_pos(pawn).unwrap();
    let site = (3..30)
        .flat_map(|r| [c.offset(r, 0), c.offset(-r, 0), c.offset(0, r), c.offset(0, -r)])
        .find(|&o| {
            (-1..=1).all(|dx| {
                (-1..=3).all(|dy| {
                    let p = o.offset(dx, dy);
                    sim.world.map.passable(p)
                        && sim.world.map.item_at(p).is_none()
                        && sim.world.map.fixture_at(p).is_none()
                })
            })
        })
        .expect("open ground");
    (sim, pawn, site, dir)
}

fn spawn(sim: &mut Sim, id: &str, at: IVec) -> Entity {
    let def = sim.world.defs.thing_id(id).unwrap();
    sim.world.spawn_fixture_of(def, at, false, None).expect("placed")
}

fn contents(sim: &Sim, store: Entity) -> Vec<(DefId, u32)> {
    let st = sim.world.ecs.get::<&Store>(store).unwrap().clone();
    st.slots.iter().flatten().map(|&e| sim.world.thing(e).map(|t| (t.def, t.count)).unwrap()).collect()
}

fn run(sim: &mut Sim, ticks: u32) {
    for _ in 0..ticks {
        sim.step();
    }
}

#[test]
fn a_crate_holds_four_stacks_in_one_cell_and_refuses_bulky_ones() {
    let (mut sim, pawn, site, dir) = colony("containers-four");
    let defs = sim.world.defs.clone();
    let crate_ = spawn(&mut sim, "boxes:crate", site);
    // Nothing edible: the colonist would eat it before it was stored.
    let (berries, flint, fibre, bone, wood) = (
        defs.thing_id("cordage").unwrap(),
        defs.thing_id("flint").unwrap(),
        defs.thing_id("fibre").unwrap(),
        defs.thing_id("bone").unwrap(),
        defs.thing_id("wood").unwrap(),
    );
    let at = sim.world.pawn_pos(pawn).unwrap();
    for (i, d) in [berries, flint, fibre, bone, wood].into_iter().enumerate() {
        sim.world.place_item(d, at.offset(-2, i as i32 - 2), 10);
    }
    run(&mut sim, 4_000);
    let mut held: Vec<DefId> = contents(&sim, crate_).into_iter().map(|(d, _)| d).collect();
    held.sort();
    let mut want = vec![berries, flint, fibre, bone];
    want.sort();
    assert_eq!(held, want, "four stacks, one cell");
    assert_eq!(sim.world.map.item_at(site), None, "nothing on the item layer under it");
    assert!(!contents(&sim, crate_).iter().any(|&(d, _)| d == wood), "wood is bulky");
    assert_eq!(sim.world.stock.on_map(wood), 10, "and still lies where it was");
    assert_eq!(sim.world.stock.stored(berries), 10, "stored counts what's in containers");
    assert_eq!(sim.world.stock, sim.world.counted_stock());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn building_deliveries_take_from_a_container() {
    let (mut sim, _, site, dir) = colony("containers-deliver");
    let defs = sim.world.defs.clone();
    let (wood, wall) = (defs.thing_id("wood").unwrap(), defs.thing_id("wall").unwrap());
    let bx = spawn(&mut sim, "boxes:box", site);
    // Every piece of wood the colony has is in the box.
    let loose: Vec<Entity> =
        sim.world.ecs.query::<(Entity, &Thing)>().iter().filter(|(_, t)| t.def == wood).map(|(e, _)| e).collect();
    for e in loose {
        let _ = sim.world.pick_up(e, u32::MAX);
    }
    assert_eq!(sim.world.put_in_store(bx, Lot::new(wood, 20)), 0);
    let at = site.offset(0, 3);
    sim.push(Command::Build { thing: wall, stuff: Some(wood), a: at, b: at });
    run(&mut sim, 6_000);
    let built =
        sim.world.map.fixture_at(at).is_some_and(|f| sim.world.ecs.get::<&rim_sim::world::Blueprint>(f).is_err());
    assert!(built, "the wall went up from the box's wood");
    let left: u32 = contents(&sim, bx).iter().map(|&(_, n)| n).sum();
    assert_eq!(left, 20 - 5, "five came out of the box");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn tearing_down_a_full_container_loses_nothing() {
    let (mut sim, _, site, dir) = colony("containers-spill");
    let defs = sim.world.defs.clone();
    let bx = spawn(&mut sim, "boxes:box", site);
    let (flint, bone) = (defs.thing_id("flint").unwrap(), defs.thing_id("bone").unwrap());
    sim.world.put_in_store(bx, Lot::new(flint, 100));
    sim.world.put_in_store(bx, Lot::new(bone, 60));
    let before = (sim.world.stock.on_map(flint), sim.world.stock.on_map(bone));
    sim.world.despawn_thing(bx);
    assert_eq!((sim.world.stock.on_map(flint), sim.world.stock.on_map(bone)), before, "all set down nearby");
    assert_eq!(sim.world.ecs.query::<&Contained>().iter().count(), 0, "nothing left inside anything");
    assert_eq!(sim.world.stock, sim.world.counted_stock());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_container_is_sorted_by_level_and_its_filter_like_a_zone() {
    let (mut sim, pawn, site, dir) = colony("containers-level");
    let defs = sim.world.defs.clone();
    let flint = defs.thing_id("flint").unwrap();
    let low = spawn(&mut sim, "boxes:crate", site);
    let high = spawn(&mut sim, "boxes:crate", site.offset(0, 2));
    sim.world.put_in_store(low, Lot::new(flint, 12));
    sim.push(Command::StoreLevel { store: StoreRef::Thing(high), level: 2 });
    run(&mut sim, 3_000);
    assert!(contents(&sim, low).is_empty() && contents(&sim, high) == [(flint, 12)], "it climbed");
    // The high crate stops taking flint: it goes back down.
    sim.push(Command::StoreFilter {
        store: StoreRef::Thing(high),
        edit: FilterEdit::Thing { thing: flint, on: false },
    });
    run(&mut sim, 3_000);
    assert_eq!(contents(&sim, low), [(flint, 12)]);
    // A filter never takes more than the def can: the crate can't take wood.
    let wood = defs.thing_id("wood").unwrap();
    sim.push(Command::StoreFilter { store: StoreRef::Thing(high), edit: FilterEdit::Thing { thing: wood, on: true } });
    sim.step();
    assert!(!sim.world.ecs.get::<&Store>(high).unwrap().filter.allows.contains(&wood));
    let _ = pawn;
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn containers_survive_a_save_to_the_byte() {
    let (mut sim, _, site, dir) = colony("containers-save");
    let defs = sim.world.defs.clone();
    let bx = spawn(&mut sim, "boxes:box", site);
    sim.world.put_in_store(bx, Lot::new(defs.thing_id("flint").unwrap(), 30));
    sim.world
        .put_in_store(bx, Lot { made_of: defs.thing_id("flint"), ..Lot::new(defs.thing_id("hand_axe").unwrap(), 1) });
    sim.push(Command::StoreLevel { store: StoreRef::Thing(bx), level: 3 });
    run(&mut sim, 50);
    let snap = Snapshot::capture(&sim);
    let back = Snapshot::from_bytes(&snap.to_bytes()).unwrap().restore(&dir, &|_| true).unwrap();
    assert_eq!(Snapshot::capture(&back).to_bytes(), snap.to_bytes(), "save, load, save: the same bytes");
    assert_eq!(back.world.stock, sim.world.stock);
    assert_eq!(back.world.stores, sim.world.stores);
    assert_eq!(back.world.map.item_at(site), None, "contents stay off the item layer");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_container_whose_mod_is_gone_sets_its_contents_down() {
    let (mut sim, _, site, dir) = colony("containers-gone");
    let flint = sim.world.defs.thing_id("flint").unwrap();
    let bx = spawn(&mut sim, "boxes:box", site);
    sim.world.put_in_store(bx, Lot::new(flint, 30));
    let before = sim.world.stock.on_map(flint);
    let without = common::test_mods("containers-gone-without", &["core", "crafting", "primitive"], &[]);
    let (back, _) = Snapshot::capture(&sim).restore_noting(&without, &|_| true).unwrap();
    let flint = back.world.defs.thing_id("flint").unwrap();
    assert_eq!(back.world.stock.on_map(flint), before, "the flint is on the ground, not gone");
    assert_eq!(back.world.stock, back.world.counted_stock());
    for d in [dir, without] {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
fn the_ledger_and_the_store_index_hold_through_a_game_with_containers() {
    let (mut sim, pawn, site, dir) = colony("containers-index");
    let defs = sim.world.defs.clone();
    spawn(&mut sim, "boxes:crate", site);
    spawn(&mut sim, "boxes:box", site.offset(0, 2));
    let at = sim.world.pawn_pos(pawn).unwrap();
    for (i, id) in ["flint", "berries", "fibre", "bone", "stones"].iter().enumerate() {
        sim.world.place_item(defs.thing_id(id).unwrap(), at.offset(-3, i as i32 - 2), 15);
    }
    for step in 0..3_000 {
        sim.step();
        if step % 250 == 0 {
            assert_eq!(sim.world.stock, sim.world.counted_stock(), "ledger at {step}");
            let kept = sim.world.stores.clone();
            sim.world.rebuild_stores();
            assert_eq!(kept, sim.world.stores, "index at {step}");
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn scripts_read_fill_and_empty_containers() {
    let script = r#"
        rim.on("probe:go", function(e)
            local left = rim.store_put(e.id, { thing = "primitive:flint", count = 40 })
            local refused = rim.store_put(e.id, { thing = "core:wood", count = 5 })
            local taken = rim.store_take(e.id, 1, 15)
            local s = rim.store(e.id)
            rim.set_data("probe:r", { left = left, refused = refused, taken = taken, slots = s.slots, count = s.contents[1].count, thing = s.contents[1].thing })
        end)
    "#;
    let dir = common::test_mods(
        "containers-script",
        &["core", "crafting", "primitive"],
        &[("boxes", &[("defs/boxes.toml", DEFS)]), ("probe", &[("scripts/probe.luau", script)])],
    );
    let mut sim = Sim::with_mods(&dir, 5, &|_| true).unwrap();
    let c = sim.world.colony_center().unwrap();
    let open = (2..30)
        .map(|r| c.offset(r, r))
        .find(|&p| sim.world.map.passable(p) && sim.world.map.fixture_at(p).is_none())
        .unwrap();
    let def = sim.world.defs.thing_id("boxes:crate").unwrap();
    let e = sim.world.spawn_fixture_of(def, open, false, None).unwrap();
    use rim_sim::data::{Data, Key};
    let data = Data::Table([(Key::Str("id".into()), Data::Int(e.to_bits().get() as i64))].into_iter().collect());
    sim.push(Command::ModEvent { name: "probe:go".into(), data: Some(data) });
    run(&mut sim, 2);
    let r = sim.world.data.get("probe:r").unwrap_or_else(|| panic!("{:?}", sim.world.messages));
    let num = |k: &str| r.get(k).and_then(|d| d.num()).unwrap_or(-1.0);
    assert_eq!(
        (num("left"), num("refused"), num("taken"), num("slots"), num("count")),
        (0.0, 5.0, 15.0, 4.0, 25.0),
        "{r:?}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_bill_takes_its_inputs_from_a_container() {
    use rim_sim::data::{Data, Key};
    let (mut sim, pawn, site, dir) = colony("containers-bill");
    let defs = sim.world.defs.clone();
    let (fibre, cordage) = (defs.thing_id("fibre").unwrap(), defs.thing_id("cordage").unwrap());
    for (w, d) in defs.work_types.iter().enumerate() {
        if d.id == "crafting:craft" {
            sim.push(Command::SetPriority { pawn, work: w as DefId, level: 1 });
        }
    }
    // Every piece of fibre is in the box.
    let loose: Vec<Entity> =
        sim.world.ecs.query::<(Entity, &Thing)>().iter().filter(|(_, t)| t.def == fibre).map(|(e, _)| e).collect();
    for e in loose {
        let _ = sim.world.pick_up(e, u32::MAX);
    }
    let bx = spawn(&mut sim, "boxes:box", site);
    sim.world.put_in_store(bx, Lot::new(fibre, 9));
    let spot = spawn(&mut sim, "crafting:spot", site.offset(0, 2));
    let data = Data::Table(
        [
            (Key::Str("site".into()), Data::Int(spot.to_bits().get() as i64)),
            (Key::Str("recipe".into()), Data::Str("primitive:cordage".into())),
        ]
        .into_iter()
        .collect(),
    );
    sim.push(Command::ModEvent { name: "crafting:add_bill".into(), data: Some(data) });
    run(&mut sim, 6_000);
    assert!(sim.world.stock.on_map(cordage) >= 1, "cordage was made");
    let left: u32 = contents(&sim, bx).iter().filter(|&&(d, _)| d == fibre).map(|&(_, n)| n).sum();
    assert_eq!(left, 9 - 3, "its three fibre came out of the box");
    let _ = std::fs::remove_dir_all(dir);
}
