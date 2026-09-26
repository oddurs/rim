//! Saves as text: unpack, pack and diff (DESIGN.md §7a).

mod common;

use rim_sim::data::{Data, Key};
use rim_sim::savefile::{self, Root, SaveFile};
use rim_sim::savetext;
use rim_sim::snapshot::Snapshot;
use rim_sim::{Command, Sim};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn temp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("rim-savetext-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    let _ = fs::remove_file(&p);
    p
}

/// Script data in every shape the text form has to tell apart.
fn odd_data() -> Data {
    let t = |kv: Vec<(Key, Data)>| Data::Table(kv.into_iter().collect());
    let s = |x: &str| Data::Str(x.into());
    t(vec![
        (Key::Str("list".into()), t(vec![(Key::Int(1), Data::Int(1)), (Key::Int(2), Data::Num(1.0))])),
        (Key::Str("sparse".into()), t(vec![(Key::Int(2), s("b")), (Key::Str("2".into()), s("two"))])),
        (Key::Str("hash".into()), t(vec![(Key::Str("#".into()), t(vec![(Key::Int(1), s("x"))]))])),
        (Key::Str("empty".into()), t(vec![])),
        (Key::Str("third".into()), Data::Num(1.0 / 3.0)),
        // A record too long for one line, of numbers only.
        (
            Key::Str("record".into()),
            t((0..9).map(|i| (Key::Str(format!("k{i}")), Data::Num(i as f64 / 7.0))).collect()),
        ),
    ])
}

/// A game with commands in its log and two snapshots, saved at `path`.
fn a_save(mods: &Path, path: &Path) {
    let mut sim = Sim::new(mods, 3).expect("mods load");
    common::arm(&mut sim);
    sim.world.data.insert("test:odd".into(), odd_data());
    let mut save = SaveFile::create(path, &mut sim).expect("save created");
    let chop = sim.world.defs.lookup("designation", "chop").unwrap();
    let wall = sim.world.defs.thing_id("wall").unwrap();
    let c = sim.world.colony_center().unwrap();
    sim.push(Command::Designate { designation: chop, a: c.offset(-10, -10), b: c.offset(10, 10) });
    sim.push(Command::Build {
        stuff: sim.world.defs.thing_id("wood"),
        thing: wall,
        a: c.offset(2, 2),
        b: c.offset(6, 2),
    });
    for _ in 0..3 {
        for _ in 0..600 {
            sim.step();
        }
        save.log(&mut sim).unwrap();
    }
    save.snapshot(&mut sim).unwrap();
    for _ in 0..600 {
        sim.step();
    }
    save.log(&mut sim).unwrap();
}

fn load(path: &Path, mods: &Path) -> (Sim, savefile::LoadReport) {
    let (sim, _, report) = SaveFile::load(path, mods, &|_| true).unwrap_or_else(|e| panic!("loads: {e}"));
    (sim, report)
}

#[test]
fn unpack_then_pack_loads_to_the_same_state() {
    let (mods, save, dir, back) = (common::mods(), temp("a.rim"), temp("a"), temp("a-back.rim"));
    a_save(&mods, &save);
    let unpacked = savetext::unpack(&save, &dir).unwrap_or_else(|e| panic!("unpacks: {e}"));
    assert_eq!((unpacked.epochs, unpacked.snapshots, unpacked.cut), (1, 2, 0));
    assert_eq!(savetext::pack(&dir, &back).unwrap_or_else(|e| panic!("packs: {e}")), None, "nothing was edited");

    let ((was, _), (now, _)) = (savefile::read(&save).unwrap(), savefile::read(&back).unwrap());
    assert_eq!(was[0].epoch, now[0].epoch);
    assert_eq!(was[0].snapshots, now[0].snapshots);
    assert_eq!(was[0].logs, now[0].logs);
    let ((a, ra), (b, rb)) = (load(&save, &mods), load(&back, &mods));
    assert_eq!((ra.tick, ra.new_epoch), (rb.tick, rb.new_epoch));
    assert_eq!(Snapshot::capture(&a).hash(), Snapshot::capture(&b).hash());
    assert_eq!(b.world.data["test:odd"], odd_data());

    // Readable: defs by name, the map drawn, data as a script wrote it.
    let snap = dir.join(format!("epoch-0/tick-{}", was[0].snapshots[1].header.tick));
    let pawns = fs::read_to_string(snap.join("engine/pawn.json")).unwrap();
    assert!(pawns.contains("\"def\": \"core:"), "{}", &pawns[..400]);
    let map: Value = serde_json::from_str(&fs::read_to_string(snap.join("engine/map.json")).unwrap()).unwrap();
    assert_eq!(map["rows"].as_array().unwrap().len(), a.world.map.h as usize);
    let log = fs::read_to_string(dir.join("epoch-0/log.json")).unwrap();
    assert!(log.contains("\"thing\": \"core:wall\""), "{log}");
    let data: Value = serde_json::from_str(&fs::read_to_string(snap.join("test/data.json")).unwrap()).unwrap();
    let odd = &data["odd"];
    assert_eq!(odd["list"], serde_json::json!([1, 1.0]));
    assert_eq!(odd["sparse"], serde_json::json!({"#": [[2, "b"], ["2", "two"]]}));
    assert_eq!(odd["hash"], serde_json::json!({"#": [["#", ["x"]]]}));
    assert_eq!(odd["empty"], serde_json::json!({}));
    for p in [&save, &back] {
        let _ = fs::remove_file(p);
    }
    let _ = fs::remove_dir_all(dir);
}

/// The snapshot directories of an unpacked save's first epoch, by tick.
fn snapshot_dirs(dir: &Path) -> Vec<PathBuf> {
    let tick =
        |p: &PathBuf| p.file_name().unwrap().to_str().unwrap().trim_start_matches("tick-").parse::<u64>().unwrap();
    let mut dirs: Vec<PathBuf> =
        fs::read_dir(dir.join("epoch-0")).unwrap().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort_by_key(tick);
    dirs
}

fn edit_json(file: &Path, f: impl FnOnce(&mut Value)) {
    let mut v: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
    f(&mut v);
    fs::write(file, v.to_string()).unwrap();
}

/// Rename the first pawn in a snapshot directory; its entity id.
fn rename_a_pawn(snap: &Path, name: &str) -> u64 {
    let mut id = 0;
    edit_json(&snap.join("engine/pawn.json"), |pawns| {
        id = pawns[0][0].as_u64().unwrap();
        pawns[0][1]["name"] = name.into();
    });
    id
}

#[test]
fn an_edited_snapshot_packs_into_a_new_epoch() {
    let (mods, save, dir, back) = (common::mods(), temp("b.rim"), temp("b"), temp("b-back.rim"));
    a_save(&mods, &save);
    savetext::unpack(&save, &dir).unwrap();
    rename_a_pawn(snapshot_dirs(&dir).last().unwrap(), "Edited");
    let tick = savetext::pack(&dir, &back).unwrap().expect("the edit starts an epoch");

    let ((was, _), (now, _)) = (savefile::read(&save).unwrap(), savefile::read(&back).unwrap());
    assert_eq!(now.len(), 2);
    assert_eq!(now[0].logs, was[0].logs, "the old log is kept as history");
    assert_eq!(now[1].epoch.root, Root::Snapshot);
    assert_eq!((now[1].snapshots.len(), now[1].logs.len(), now[1].snapshots[0].header.tick), (1, 0, tick));
    let (sim, report) = load(&back, &mods);
    assert_eq!((report.tick, report.new_epoch), (tick, None));
    assert!(sim.world.ecs.query::<&rim_sim::world::Pawn>().iter().any(|p| p.name == "Edited"));

    // Two edits at once would leave two roots.
    rename_a_pawn(&snapshot_dirs(&dir)[0], "Also edited");
    let err = savetext::pack(&dir, &back).unwrap_err();
    assert!(err.contains("edit one at a time"), "{err}");
    for p in [&save, &back] {
        let _ = fs::remove_file(p);
    }
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn diff_names_the_section_and_entity_of_the_first_difference() {
    let (mods, save, dir, back) = (common::mods(), temp("c.rim"), temp("c"), temp("c-back.rim"));
    a_save(&mods, &save);
    assert_eq!(savetext::diff(&save, &save).unwrap(), Vec::<String>::new());
    savetext::unpack(&save, &dir).unwrap();
    let snap = snapshot_dirs(&dir).pop().unwrap();
    let id = rename_a_pawn(&snap, "Edited");
    // And one cell of the map: the first cell of row 3 becomes another terrain.
    edit_json(&snap.join("engine/map.json"), |map| {
        let row = map["rows"][3].as_str().unwrap().to_string();
        let other = map["legend"].as_object().unwrap().keys().find(|g| !row.starts_with(g.as_str())).unwrap().clone();
        map["rows"][3] = format!("{other}{}", &row[1..]).into();
    });
    savetext::pack(&dir, &back).unwrap();

    let found = savetext::diff(&save, &back).unwrap();
    assert!(
        found.contains(&format!("engine:pawn: entity {id}.name: \"{}\" ≠ \"Edited\"", pawn_name(&save, id))),
        "{found:?}"
    );
    assert!(found.iter().any(|l| l.starts_with("engine:map: rows[3], column 0: ")), "{found:?}");
    for p in [&save, &back] {
        let _ = fs::remove_file(p);
    }
    let _ = fs::remove_dir_all(dir);
}

fn pawn_name(save: &Path, id: u64) -> String {
    let (epochs, _) = savefile::read(save).unwrap();
    let snap = epochs[0].snapshots.last().unwrap();
    let pawns: Vec<(rim_sim::hecs::Entity, rim_sim::world::Pawn)> =
        rmp_serde::from_slice(&snap.sections["engine:pawn"]).unwrap();
    pawns.into_iter().find(|p| p.0.to_bits().get() == id).unwrap().1.name
}
