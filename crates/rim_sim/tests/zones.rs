//! Stockpile zones: painted by commands, filtered by item, saved.

mod common;

use rim_sim::snapshot::Snapshot;
use rim_sim::{Command, IVec, Sim};

fn sim() -> Sim {
    Sim::new(&common::mods(), 4).unwrap()
}

#[test]
fn zones_are_painted_extended_and_cleared_by_commands() {
    let mut s = sim();
    let c = s.world.colony_center().unwrap();
    s.push(Command::Stockpile { a: c, b: c.offset(2, 1), zone: None });
    s.step();
    let z = s.world.zones.list[0].clone();
    assert_eq!(z.name, "Stockpile 1");
    let wood = s.world.defs.thing_id("wood").unwrap();
    assert!(z.takes(wood), "a new zone takes every item");
    assert_eq!(s.world.zones.cells.iter().filter(|&&c| c == z.id).count(), 6);

    // A rectangle touching it extends it; the client asks `touched`.
    let touched = s.world.zones.touched(&s.world.map, c.offset(2, 0), c.offset(4, 0));
    assert_eq!(touched, Some(z.id));
    s.push(Command::Stockpile { a: c.offset(2, 0), b: c.offset(4, 0), zone: touched });
    s.push(Command::ZoneAllow { zone: z.id, thing: wood, on: false });
    s.step();
    assert_eq!(s.world.zones.cells.iter().filter(|&&c| c == z.id).count(), 8);
    assert!(!s.world.zones.get(z.id).unwrap().takes(wood));
    assert_eq!(s.world.zones.at(&s.world.map, c.offset(4, 0)).map(|z| z.id), Some(z.id));

    // Clearing every cell removes the zone; ids aren't reused.
    s.push(Command::ClearZone { a: c.offset(-1, -1), b: c.offset(5, 2) });
    s.push(Command::Stockpile { a: c.offset(10, 10), b: c.offset(10, 10), zone: None });
    s.step();
    let ids: Vec<u32> = s.world.zones.list.iter().map(|z| z.id).collect();
    assert_eq!(ids, [2]);
    assert!(s.world.zones.at(&s.world.map, c).is_none());
    assert!(s.world.zones.at(&s.world.map, IVec::new(-1, -1)).is_none(), "off the map is no zone");
}

#[test]
fn zones_survive_a_save() {
    let mut s = sim();
    let c = s.world.colony_center().unwrap();
    let stone = s.world.defs.thing_id("stone").unwrap();
    s.push(Command::Stockpile { a: c, b: c.offset(3, 3), zone: None });
    s.push(Command::ZoneAllow { zone: 1, thing: stone, on: false });
    s.step();
    let back = Snapshot::capture(&s).restore(&common::mods(), &|_| true).unwrap();
    assert_eq!(back.world.zones, s.world.zones);
    assert_eq!(back.world.state_hash(), s.world.state_hash());
}

#[test]
fn a_removed_mods_items_leave_the_filters_with_a_note() {
    let defs = r##"
[[thing]]
id = "shell"
label = "shell"
color = "#e0d0c0"
category = "item"
market_value = 1
stack_limit = 50
"##;
    let with = common::test_mods("zones-with", &["core"], &[("shells", &[("defs/items.toml", defs)])]);
    let without = common::test_mods("zones-without", &["core"], &[]);
    let mut s = Sim::with_mods(&with, 4, &|_| true).unwrap();
    let c = s.world.colony_center().unwrap();
    s.push(Command::Stockpile { a: c, b: c.offset(1, 1), zone: None });
    s.step();
    let shell = s.world.defs.thing_id("shells:shell").unwrap();
    assert!(s.world.zones.list[0].takes(shell));
    let (back, notes) = Snapshot::capture(&s).restore_noting(&without, &|_| true).unwrap();
    assert!(notes.iter().any(|n| n.contains("shells:shell")), "{notes:?}");
    let wood = back.world.defs.thing_id("wood").unwrap();
    assert!(back.world.zones.list[0].takes(wood), "the rest of the filter maps by id");
    for d in [with, without] {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
fn painting_never_takes_cells_from_another_zone_and_only_items_are_allowed() {
    let mut s = sim();
    let c = s.world.colony_center().unwrap();
    s.push(Command::Stockpile { a: c, b: c.offset(1, 0), zone: None });
    s.push(Command::Stockpile { a: c.offset(3, 0), b: c.offset(4, 0), zone: None });
    s.step();
    // A drag over both makes a third zone of the free cells between them.
    s.push(Command::Stockpile { a: c, b: c.offset(4, 0), zone: None });
    let wall = s.world.defs.thing_id("wall").unwrap();
    s.push(Command::ZoneAllow { zone: 1, thing: wall, on: true });
    s.step();
    let at = |s: &Sim, dx| s.world.zones.at(&s.world.map, c.offset(dx, 0)).map(|z| z.id);
    assert_eq!((at(&s, 0), at(&s, 1), at(&s, 2), at(&s, 3), at(&s, 4)), (Some(1), Some(1), Some(3), Some(2), Some(2)));
    assert!(!s.world.zones.get(1).unwrap().takes(wall), "a stockpile only takes items");
}

#[test]
fn a_loaded_set_of_zones_is_made_consistent() {
    let mut z = rim_sim::zone::Zones::new(4);
    z.list.push(rim_sim::zone::Zone { id: 7, name: "a".into(), allows: vec![3, 1, 3] });
    z.list.push(rim_sim::zone::Zone { id: 7, name: "twin".into(), allows: vec![] });
    z.list.push(rim_sim::zone::Zone { id: 9, name: "empty".into(), allows: vec![] });
    z.cells = vec![7, 5, 0, 7];
    z.next_id = 2;
    z.tidy();
    assert_eq!(z.list.len(), 1, "no duplicate id, no zone without cells");
    assert_eq!(z.list[0].allows, [1, 3]);
    assert_eq!(z.cells, [7, 0, 0, 7], "cells of a missing zone are freed");
    assert_eq!(z.next_id, 10, "past every id the save named");
}
