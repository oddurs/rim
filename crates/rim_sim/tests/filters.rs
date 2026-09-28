//! Item categories and the one filter stores use (DESIGN.md §4f): which
//! things, not made of which materials, in what condition.

mod common;

use rim_sim::defs::{Category, DefDb, DefId};
use rim_sim::filter::{Filter, FilterEdit};
use rim_sim::snapshot::Snapshot;
use rim_sim::world::{Lot, Thing};
use rim_sim::zone::StoreRef;
use rim_sim::{Command, IVec, Sim};

fn category(defs: &DefDb, id: &str) -> DefId {
    defs.item_categories.iter().position(|c| c.id == id).unwrap_or_else(|| panic!("no category {id}")) as DefId
}

fn ids(defs: &DefDb, v: &[DefId]) -> Vec<String> {
    let mut v: Vec<String> = v.iter().map(|&d| defs.thing(d).id.clone()).collect();
    v.sort();
    v
}

#[test]
fn every_item_lands_in_the_categories_it_should() {
    let sim = Sim::new(&common::mods(), 1).unwrap();
    let defs = &sim.world.defs;
    let direct = |id: &str| ids(defs, &defs.item_categories[category(defs, id) as usize].items);
    assert_eq!(direct("core:food"), ["core:berries", "core:raw_meat", "crafting:roast_meat", "primitive:stew"]);
    assert_eq!(
        direct("core:materials"),
        ["core:stone", "core:wood", "primitive:branches", "primitive:clay", "timber:planks"]
    );
    assert_eq!(direct("primitive:knapping"), ["primitive:bone", "primitive:flint"]);
    assert_eq!(direct("primitive:fibres"), ["primitive:cordage", "primitive:fibre"]);
    assert_eq!(direct("primitive:stones"), ["primitive:stones"]);
    assert_eq!(direct("primitive:vessels"), ["primitive:pot"]);
    assert!(direct("core:tools").contains(&"primitive:hand_axe".to_string()));
    // Materials holds its children's items too.
    let under = ids(defs, &defs.category_items(category(defs, "core:materials")));
    assert!(under.contains(&"primitive:flint".to_string()) && under.contains(&"core:wood".to_string()));
    // Every item is somewhere, so every item can be filtered.
    let everywhere: Vec<DefId> = defs.item_categories.iter().flat_map(|c| c.items.iter().copied()).collect();
    for (d, t) in defs.things.iter().enumerate() {
        if t.category == Category::Item {
            assert!(everywhere.contains(&(d as DefId)), "{} is in no category", t.id);
        }
    }
    // The top level is core's four, in order, and children follow `order`.
    let roots: Vec<&str> = defs.category_roots.iter().map(|&c| defs.item_categories[c as usize].id.as_str()).collect();
    assert_eq!(roots, ["core:food", "core:materials", "core:tools", "primitive:vessels", "core:other"]);
    let kids: Vec<&str> = defs.item_categories[category(defs, "core:materials") as usize]
        .children
        .iter()
        .map(|&c| defs.item_categories[c as usize].id.as_str())
        .collect();
    assert_eq!(kids, ["primitive:knapping", "primitive:stones", "primitive:fibres", "iron:metal"]);
}

#[test]
fn a_filter_takes_by_thing_material_and_condition() {
    let sim = Sim::new(&common::mods(), 1).unwrap();
    let defs = &sim.world.defs;
    let (axe, flint, bone) =
        (defs.thing_id("hand_axe").unwrap(), defs.thing_id("flint").unwrap(), defs.thing_id("bone").unwrap());
    let mut f = Filter::everything(defs);
    let full = defs.full_hp(axe, Some(flint));
    assert!(f.takes(defs, axe, Some(bone), None));
    f.edit(defs, FilterEdit::Material { material: bone, on: false });
    assert!(!f.takes(defs, axe, Some(bone), None), "no bone");
    assert!(f.takes(defs, axe, Some(flint), None), "flint still");
    f.edit(defs, FilterEdit::Condition { min: 50, max: 100 });
    assert!(f.takes(defs, axe, Some(flint), Some(full / 2 + 1)));
    assert!(!f.takes(defs, axe, Some(flint), Some(full / 2 - 1)), "worn below half");
    // A category edit covers the items under it; a non-material is no refusal.
    f.edit(defs, FilterEdit::Category { category: category(defs, "core:tools"), on: false });
    assert!(!f.takes_thing(axe));
    let berries = defs.thing_id("berries").unwrap();
    f.edit(defs, FilterEdit::Material { material: berries, on: false });
    assert!(!f.refuses.contains(&berries), "berries aren't a material");
    f.edit(defs, FilterEdit::All { on: false });
    assert!(f.allows.is_empty());
}

/// How many of `def` made of `of` lie in zone 1.
fn in_zone(sim: &Sim, def: DefId, of: Option<DefId>) -> u32 {
    sim.world
        .ecs
        .query::<(rim_sim::hecs::Entity, &Thing)>()
        .iter()
        .filter(|(e, t)| {
            t.def == def
                && sim.world.made_of(*e) == of
                && sim.world.zones.at(&sim.world.map, t.pos).is_some_and(|z| z.id == 1)
        })
        .map(|(_, t)| t.count)
        .sum()
}

#[test]
fn a_zone_that_refuses_bone_takes_the_flint_axe_and_leaves_the_bone_one() {
    let (mut sim, pawn, site) = common::hauling_colony(5);
    let defs = sim.world.defs.clone();
    let (axe, flint, bone) =
        (defs.thing_id("hand_axe").unwrap(), defs.thing_id("flint").unwrap(), defs.thing_id("bone").unwrap());
    sim.push(Command::Stockpile { a: site, b: site.offset(2, 2), zone: None });
    sim.push(Command::StoreFilter {
        store: StoreRef::Zone(1),
        edit: FilterEdit::Material { material: bone, on: false },
    });
    sim.step();
    let at = sim.world.pawn_pos(pawn).unwrap();
    // Dropped outside the stockpile, wherever the map put it: an axe that
    // lands in the zone would count as stored without anyone hauling it.
    let free = |s: &Sim, p: IVec| {
        s.world.map.passable(p) && s.world.map.item_at(p).is_none() && s.world.zones.at(&s.world.map, p).is_none()
    };
    let mut spots = (2..30)
        .flat_map(|r| [at.offset(-r, 0), at.offset(0, -r), at.offset(r, 0), at.offset(0, r)])
        .filter(|&p| free(&sim, p));
    let (a, b) = (spots.next().unwrap(), spots.next().unwrap());
    sim.world.put_lot(Lot { def: axe, count: 1, made_of: Some(bone), hp: None }, a);
    sim.world.put_lot(Lot { def: axe, count: 1, made_of: Some(flint), hp: None }, b);
    for _ in 0..3_000 {
        sim.step();
    }
    assert_eq!(in_zone(&sim, axe, Some(flint)), 1, "the flint axe is stored");
    assert_eq!(in_zone(&sim, axe, Some(bone)), 0, "the bone one isn't");
}

#[test]
fn a_zone_refuses_stacks_below_its_condition_and_lets_them_go() {
    let (mut sim, pawn, site) = common::hauling_colony(5);
    let defs = sim.world.defs.clone();
    let (axe, flint) = (defs.thing_id("hand_axe").unwrap(), defs.thing_id("flint").unwrap());
    let full = defs.full_hp(axe, Some(flint));
    sim.push(Command::Stockpile { a: site, b: site.offset(2, 2), zone: None });
    sim.step();
    // A worn axe already in the zone, and a whole one loose.
    sim.world.put_lot(Lot { def: axe, count: 1, made_of: Some(flint), hp: Some(full / 4) }, site);
    let at = sim.world.pawn_pos(pawn).unwrap();
    sim.world.put_lot(Lot { def: axe, count: 1, made_of: Some(flint), hp: Some(full) }, at.offset(-2, 0));
    sim.push(Command::StoreFilter { store: StoreRef::Zone(1), edit: FilterEdit::Condition { min: 50, max: 100 } });
    // A second zone that takes anything, for the worn one to go to.
    sim.push(Command::Stockpile { a: site.offset(6, 0), b: site.offset(6, 0), zone: None });
    for _ in 0..4_000 {
        sim.step();
    }
    let worn_in_zone = sim.world.ecs.query::<&Thing>().iter().any(|t| {
        t.def == axe && t.hp < full / 2 && sim.world.zones.at(&sim.world.map, t.pos).is_some_and(|z| z.id == 1)
    });
    assert!(!worn_in_zone, "the worn axe left zone 1");
    assert_eq!(in_zone(&sim, axe, Some(flint)), 1, "the whole one is stored in zone 1");
}

#[test]
fn filters_survive_a_save_and_a_removed_material_refuses_nothing() {
    let shells = r##"
[[thing]]
id = "shell"
label = "shell"
color = "#e0d0c0"
category = "item"
stack_limit = 50
stuff = { categories = ["lithic"] }
"##;
    let with = common::test_mods("filters-with", &["core"], &[("shells", &[("defs/items.toml", shells)])]);
    let without = common::test_mods("filters-without", &["core"], &[]);
    let mut s = Sim::with_mods(&with, 4, &|_| true).unwrap();
    let c = s.world.colony_center().unwrap();
    let (shell, wood) = (s.world.defs.thing_id("shells:shell").unwrap(), s.world.defs.thing_id("wood").unwrap());
    s.push(Command::Stockpile { a: c, b: c.offset(1, 1), zone: None });
    s.push(Command::StoreFilter {
        store: StoreRef::Zone(1),
        edit: FilterEdit::Material { material: shell, on: false },
    });
    s.push(Command::StoreFilter { store: StoreRef::Zone(1), edit: FilterEdit::Material { material: wood, on: false } });
    s.push(Command::StoreFilter { store: StoreRef::Zone(1), edit: FilterEdit::Condition { min: 30, max: 90 } });
    s.step();
    let back = Snapshot::capture(&s).restore(&with, &|_| true).unwrap();
    assert_eq!(back.world.zones, s.world.zones);
    assert_eq!(back.world.state_hash(), s.world.state_hash());
    let (gone, _) = Snapshot::capture(&s).restore_noting(&without, &|_| true).unwrap();
    let wood = gone.world.defs.thing_id("wood").unwrap();
    assert_eq!(gone.world.zones.list[0].filter.refuses, [wood], "the shell refusal is gone, wood's remapped");
    assert_eq!(gone.world.zones.list[0].filter.hp, [30, 90]);
    for d in [with, without] {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
fn a_zone_saved_before_filters_loads_taking_any_material() {
    let old = r#"{"id":3,"name":"Stockpile 3","allows":[2,1]}"#;
    let z: rim_sim::zone::Zone = serde_json::from_str(old).unwrap();
    assert_eq!((z.filter.allows.as_slice(), z.filter.refuses.len(), z.filter.hp), (&[2, 1][..], 0, [0, 100]));
    let json = serde_json::to_string(&z).unwrap();
    assert_eq!(json, old, "an unchanged filter writes what it read");
}

#[test]
fn a_category_tree_that_is_not_a_tree_or_names_a_non_item_fails_to_load() {
    let cases = [
        (
            "cycle",
            "[[item_category]]\nid = \"a\"\nlabel = \"A\"\nparent = \"b\"\n[[item_category]]\nid = \"b\"\nlabel = \"B\"\nparent = \"a\"\n",
            "own ancestor",
        ),
        ("with", "[[item_category]]\nid = \"a\"\nlabel = \"A\"\nwith = [\"hat\"]\n", "`with` names \"hat\""),
        ("wall", "[[item_category]]\nid = \"a\"\nlabel = \"A\"\nthings = [\"core:wall\"]\n", "isn't an item"),
        ("rest", "[[item_category]]\nid = \"a\"\nlabel = \"A\"\nrest = true\n", "only one category takes the rest"),
    ];
    for (name, defs, want) in cases {
        let dir = common::test_mods(&format!("cat-{name}"), &["core"], &[("bad", &[("defs/c.toml", defs)])]);
        let err = Sim::with_mods(&dir, 1, &|_| true).err().unwrap_or_else(|| panic!("{name} loaded"));
        assert!(err.contains(want), "{name}: {err}");
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn a_filter_command_on_a_zone_that_is_gone_changes_nothing() {
    let mut s = Sim::new(&common::mods(), 2).unwrap();
    let c: IVec = s.world.colony_center().unwrap();
    s.push(Command::Stockpile { a: c, b: c, zone: None });
    s.push(Command::StoreFilter { store: StoreRef::Zone(9), edit: FilterEdit::All { on: false } });
    s.step();
    assert!(!s.world.zones.list[0].filter.allows.is_empty());
}
