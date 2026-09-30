//! The stock ledger and holdings (stock.rs): kept as stacks change, never
//! counted when asked, and always what a count would say.

mod common;

use common::test_mods;
use rim_sim::defs::DefId;
use rim_sim::filter::FilterEdit;
use rim_sim::hecs::Entity;
use rim_sim::path::Goal;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{Lot, Thing, World};
use rim_sim::zone::StoreRef;
use rim_sim::{Command, IVec, Sim};

/// The old way: walk every stack.
fn nearest_by_walking(w: &World, e: Entity, from: IVec, def: DefId) -> Option<(u32, Entity)> {
    let mut best: Option<(u32, Entity)> = None;
    for (te, t) in w.ecs.query::<(Entity, &Thing)>().iter() {
        if t.def != def || w.map.item_at(t.pos) != Some(te) {
            continue;
        }
        let d = t.pos.octile(from);
        if best.is_some_and(|b| (b.0, b.1.id()) <= (d, te.id()))
            || w.reserved_by_other(te, e)
            || !w.map.can_reach(from, Goal::Cell(t.pos))
        {
            continue;
        }
        best = Some((d, te));
    }
    best
}

#[test]
fn the_ledger_is_what_a_count_says_through_a_working_game() {
    let (mut sim, pawn, site) = common::hauling_colony(7);
    let defs = sim.world.defs.clone();
    let (wood, stone) = (defs.thing_id("wood").unwrap(), defs.thing_id("stone").unwrap());
    let at = sim.world.pawn_pos(pawn).unwrap();
    for (i, d) in [wood, stone, wood, stone].into_iter().enumerate() {
        sim.world.place_item(d, at.offset(-4 - i as i32 * 3, 2), 40);
    }
    sim.push(Command::Stockpile { a: site, b: site.offset(2, 2), zone: None });
    assert_eq!(sim.world.stock, sim.world.counted_stock());
    for step in 0..4_000 {
        sim.step();
        if step == 1_500 {
            // Zones changing move stacks in and out of being stored.
            sim.push(Command::StoreFilter {
                store: StoreRef::Zone(1),
                edit: FilterEdit::Thing { thing: stone, on: false },
            });
        }
        if step % 250 == 0 {
            assert_eq!(sim.world.stock, sim.world.counted_stock(), "at step {step}");
        }
    }
    assert!(sim.world.stock.stored(wood) > 0, "some wood was stored");
    assert_eq!(sim.world.stock.stored(stone), 0, "stone isn't kept once the zone refuses it");
    assert_eq!(sim.world.stock.on_map(wood), 80);
}

#[test]
fn the_ledger_is_rebuilt_on_load() {
    let (mut sim, pawn, site) = common::hauling_colony(8);
    let wood = sim.world.defs.thing_id("wood").unwrap();
    sim.world.place_item(wood, sim.world.pawn_pos(pawn).unwrap().offset(-3, 0), 60);
    sim.push(Command::Stockpile { a: site, b: site.offset(1, 1), zone: None });
    for _ in 0..1_500 {
        sim.step();
    }
    let back = Snapshot::capture(&sim).restore(&common::mods(), &|_| true).unwrap();
    assert_eq!(back.world.stock, sim.world.stock);
    assert_eq!(back.world.stock, back.world.counted_stock());
}

#[test]
fn the_nearest_stack_is_the_one_walking_every_stack_finds() {
    let mut sim = Sim::new(&common::mods(), 11).unwrap();
    let pawn = sim.world.colonists().next().unwrap();
    let defs = sim.world.defs.clone();
    let flint = defs.thing_id("flint").unwrap();
    let mut rng = rim_sim::rng::Rng::new(99);
    let (w, h) = (sim.world.map.w, sim.world.map.h);
    for _ in 0..60 {
        let p = IVec::new(rng.range(0, w - 1), rng.range(0, h - 1));
        if sim.world.map.passable(p) {
            sim.world.put_lot(Lot::new(flint, 1 + rng.range(0, 20) as u32), p);
        }
    }
    let mut checked = 0;
    for _ in 0..300 {
        let from = IVec::new(rng.range(0, w - 1), rng.range(0, h - 1));
        if !sim.world.map.passable(from) {
            continue;
        }
        let want = nearest_by_walking(&sim.world, pawn, from, flint);
        let got = rim_sim::ai::nearest_item(&sim.world, pawn, from, flint);
        assert_eq!(got, want, "from {from:?}");
        checked += 1;
    }
    assert!(checked > 100);
}

#[test]
fn scripts_read_the_stock_by_thing_tag_and_category() {
    let script = r#"
        rim.every(10, function()
            rim.set_data("probe:r", {
                wood = rim.stock("core:wood"),
                bulky = rim.stock({ tag = "bulky" }),
                materials = rim.stock({ category = "core:materials" }),
                stored = rim.stock("core:wood", "stored"),
                loose = rim.stock("core:wood", "loose"),
                counted = rim.count_items({ thing = "core:wood" }),
            })
        end)
    "#;
    let dir = test_mods("stock-script", &["core"], &[("probe", &[("scripts/probe.luau", script)])]);
    let mut s = Sim::new(&dir, 1).unwrap();
    let c = s.world.colony_center().unwrap();
    let (wood, stone) = (s.world.defs.thing_id("wood").unwrap(), s.world.defs.thing_id("stone").unwrap());
    let before_wood = s.world.stock.on_map(wood);
    let before_stone = s.world.stock.on_map(stone);
    // A zone over one open cell with wood on it, and more wood elsewhere.
    let open = (1..30).map(|r| c.offset(r, r)).find(|&p| s.world.room_for(wood, None, p) >= 75).unwrap();
    s.world.put_lot(Lot::new(wood, 30), open);
    s.push(Command::Stockpile { a: open, b: open, zone: None });
    s.world.place_item(stone, c.offset(-5, 5), 12);
    for _ in 0..11 {
        s.step();
    }
    let r = s.world.data.get("probe:r").unwrap_or_else(|| panic!("{:?}", s.world.messages));
    let get = |k: &str| r.get(k).and_then(|d| d.num()).unwrap_or(-1.0) as u32;
    let wood_now = s.world.stock.on_map(wood);
    assert_eq!(get("wood"), wood_now);
    assert_eq!(get("counted"), wood_now);
    assert_eq!(get("bulky"), wood_now + s.world.stock.on_map(stone));
    assert_eq!(get("materials"), get("bulky"), "core's materials are wood and stone");
    assert!(get("stored") >= 30 && get("stored") + get("loose") == wood_now, "{r:?}");
    assert!(wood_now >= before_wood + 30 && s.world.stock.on_map(stone) == before_stone + 12);
    let _ = std::fs::remove_dir_all(dir);
}

/// `rim.damage` wears a stack as spoiling does: a zone that keeps it only
/// in good condition stops keeping it, and the ledger knows.
#[test]
fn a_script_damaging_a_stack_keeps_the_ledger() {
    let script = r#"
        rim.every(5, function()
            local id = rim.get_data("target")
            if id and not rim.get_data("done") then
                rim.damage(id, rim.get_data("hp"))
                rim.set_data("done", true)
            end
        end)
    "#;
    let dir = test_mods("stock-damage", &["core"], &[("probe", &[("scripts/probe.luau", script)])]);
    let mut s = Sim::new(&dir, 1).unwrap();
    let c = s.world.colony_center().unwrap();
    let wood = s.world.defs.thing_id("wood").unwrap();
    let open = (1..30).map(|r| c.offset(r, r)).find(|&p| s.world.room_for(wood, None, p) >= 75).unwrap();
    s.world.put_lot(Lot::new(wood, 30), open);
    s.push(Command::Stockpile { a: open, b: open, zone: None });
    s.push(Command::StoreFilter { store: StoreRef::Zone(1), edit: FilterEdit::Condition { min: 50, max: 100 } });
    s.step();
    let stack = s.world.map.item_at(open).unwrap();
    assert!(s.world.stock.stored(wood) >= 30, "kept while whole");
    let hp = s.world.defs.full_hp(wood, None) * 3 / 4;
    let id = rim_sim::data::Data::Int(stack.to_bits().get() as i64);
    s.world.data.insert("probe:target".into(), id);
    s.world.data.insert("probe:hp".into(), rim_sim::data::Data::Int(hp as i64));
    for _ in 0..10 {
        s.step();
    }
    assert!(s.world.data.contains_key("probe:done"), "{:?}", s.world.messages);
    assert!(s.world.thing(stack).is_some_and(|t| t.hp > 0), "worn, not destroyed");
    assert_eq!(s.world.stock, s.world.counted_stock());
    let _ = std::fs::remove_dir_all(dir);
}
