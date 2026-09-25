//! The save file: the log is the save, snapshots are a cache, and an epoch
//! starts when the code changes (DESIGN.md §7a).

mod common;

use rim_sim::savefile::{self, SaveFile};
use rim_sim::snapshot::Snapshot;
use rim_sim::{Command, Sim, TICKS_PER_DAY};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn save_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("rim-save-{name}-{}.rim", std::process::id()))
}

/// A new game with a save, and the commands a player gives on the first tick.
fn new_game(mods: &Path, path: &Path) -> (Sim, SaveFile) {
    let mut sim = Sim::new(mods, 3).expect("mods load");
    let save = SaveFile::create(path, &mut sim).expect("save created");
    let chop = sim.world.defs.lookup("designation", "chop").unwrap();
    let wall = sim.world.defs.thing_id("wall").unwrap();
    let wood = sim.world.defs.thing_id("wood").unwrap();
    let c = sim.world.colony_center().unwrap();
    sim.push(Command::Designate { designation: chop, a: c.offset(-10, -10), b: c.offset(10, 10) });
    sim.push(Command::Build { stuff: Some(wood), thing: wall, a: c.offset(2, 2), b: c.offset(6, 2) });
    (sim, save)
}

/// Play `ticks`, logging every 600, with a command now and then; the
/// snapshot hash at every log, by tick.
fn play(sim: &mut Sim, save: &mut SaveFile, ticks: u64, hashes: &mut BTreeMap<u64, u64>) {
    let wall = sim.world.defs.thing_id("wall").unwrap();
    for _ in 0..ticks {
        if sim.world.tick % 1_700 == 900 {
            let c = sim.world.colony_center().unwrap();
            let x = (sim.world.tick / 1_700) as i32 % 8;
            sim.push(Command::Build {
                stuff: sim.world.defs.thing_id("wood"),
                thing: wall,
                a: c.offset(x, 6),
                b: c.offset(x, 6),
            });
        }
        sim.step();
        if sim.world.tick.is_multiple_of(600) {
            save.log(sim).unwrap();
            hashes.insert(sim.world.tick, Snapshot::capture(sim).hash());
        }
    }
}

#[test]
fn load_is_the_newest_snapshot_and_the_log_after_it() {
    let path = save_path("replay");
    let mods = common::mods();
    let (mut sim, mut save) = new_game(&mods, &path);
    let mut hashes = BTreeMap::new();
    play(&mut sim, &mut save, TICKS_PER_DAY / 2, &mut hashes);
    save.snapshot(&mut sim).unwrap();
    let snapped = sim.world.tick;
    play(&mut sim, &mut save, TICKS_PER_DAY / 2 + 300, &mut hashes);
    // The process dies here: the last 300 ticks were never logged.
    let last_logged = *hashes.keys().last().unwrap();
    drop(save);

    let (loaded, _, report) = SaveFile::load(&path, &mods, &|_| true).unwrap_or_else(|e| panic!("loads: {e}"));
    assert_eq!(report.from_snapshot, snapped);
    assert_eq!(report.tick, last_logged, "{report:?}");
    assert_eq!(report.replayed, last_logged - snapped);
    assert_eq!(report.new_epoch, None);
    assert_eq!(Snapshot::capture(&loaded).hash(), hashes[&last_logged], "the replayed game is the live one");
    let _ = std::fs::remove_file(path);
}

#[test]
fn a_torn_file_loads_up_to_its_last_whole_chunk() {
    let path = save_path("torn");
    let mods = common::mods();
    let (mut sim, mut save) = new_game(&mods, &path);
    let mut hashes = BTreeMap::new();
    play(&mut sim, &mut save, 3_000, &mut hashes);
    drop(save);
    // Cut the file in the middle of its last chunk.
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, &bytes[..bytes.len() - 20]).unwrap();

    let (loaded, mut save, report) = SaveFile::load(&path, &mods, &|_| true).unwrap();
    assert!(report.cut > 0, "{report:?}");
    let copy = report.damaged_copy.clone().expect("the damaged file is kept");
    assert_eq!(std::fs::read(&copy).unwrap(), &bytes[..bytes.len() - 20], "as it was before the cut");
    let _ = std::fs::remove_file(copy);
    assert_eq!(report.tick, 2_400, "the last whole log");
    assert_eq!(Snapshot::capture(&loaded).hash(), hashes[&2_400]);
    // New chunks follow the whole ones, and the file reads back.
    let mut loaded = loaded;
    play(&mut loaded, &mut save, 600, &mut hashes);
    drop(save);
    let (_, _, again) = SaveFile::load(&path, &mods, &|_| true).unwrap();
    assert_eq!((again.cut, again.tick), (0, 3_000));
    let _ = std::fs::remove_file(path);
}

#[test]
fn an_unchanged_section_is_stored_once() {
    let path = save_path("dedup");
    let mods = common::mods();
    let (mut sim, mut save) = new_game(&mods, &path);
    let first = std::fs::metadata(&path).unwrap().len();
    sim.step();
    save.snapshot(&mut sim).unwrap();
    let second = std::fs::metadata(&path).unwrap().len() - first;
    // The terrain, the defs and most of the fields didn't change in a tick.
    assert!(second * 2 < first, "a snapshot one tick later added {second} bytes; the first was {first}");
    let (epochs, _) = savefile::read(&path).unwrap();
    assert_eq!(epochs[0].snapshots.len(), 2);
    assert_eq!(epochs[0].snapshots[0].sections["engine:map"], epochs[0].snapshots[1].sections["engine:map"]);
    let _ = std::fs::remove_file(path);
}

#[test]
fn a_changed_mod_list_starts_a_new_epoch() {
    let path = save_path("epoch");
    let before = common::test_mods("epoch-a", &["core", "weather"], &[]);
    let (mut sim, mut save) = new_game(&before, &path);
    let mut hashes = BTreeMap::new();
    play(&mut sim, &mut save, 1_200, &mut hashes);
    save.snapshot(&mut sim).unwrap();
    play(&mut sim, &mut save, 600, &mut hashes);
    drop(save);

    // The player adds a script-only mod: no defs change, so the snapshot
    // loads, but the log can't replay under different code.
    let script = "rim.every(100, function() rim.set_data(\"seen\", rim.tick()) end)\n";
    let after = common::test_mods("epoch-b", &["core", "weather"], &[("extra", &[("scripts/extra.luau", script)])]);
    let (loaded, save, report) = SaveFile::load(&path, &after, &|_| true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(report.new_epoch.as_deref(), Some("the mods or the engine changed"));
    assert_eq!((report.tick, report.lost), (1_200, 600), "{report:?}");
    assert!(loaded.mods.iter().any(|m| m.id == "extra"));
    drop(save);
    let (epochs, _) = savefile::read(&path).unwrap();
    assert_eq!(epochs.len(), 2);
    assert!(epochs[1].epoch.mods.iter().any(|m| m.0 == "extra"));

    // Loading again under the same mods is a plain load in the new epoch.
    let (_, _, again) = SaveFile::load(&path, &after, &|_| true).unwrap();
    assert_eq!((again.new_epoch, again.tick), (None, 1_200));
    for d in [before, after] {
        let _ = std::fs::remove_dir_all(d);
    }
    let _ = std::fs::remove_file(path);
}

#[test]
fn a_log_that_stops_replaying_resumes_before_the_divergence() {
    let path = save_path("diverge");
    let mods = common::mods();
    let (mut sim, mut save) = new_game(&mods, &path);
    let mut hashes = BTreeMap::new();
    play(&mut sim, &mut save, 1_800, &mut hashes);
    // Something the log doesn't know about changes the game: a bug.
    sim.world.rng = rim_sim::rng::Rng::from_state(12345);
    play(&mut sim, &mut save, 1_200, &mut hashes);
    drop(save);

    let (loaded, _, report) = SaveFile::load(&path, &mods, &|_| true).unwrap();
    assert_eq!(report.diverged_at, Some(2_400), "{report:?}");
    assert_eq!(report.tick, 1_800);
    assert!(report.new_epoch.is_some());
    assert_eq!(Snapshot::capture(&loaded).hash(), hashes[&1_800]);
    let _ = std::fs::remove_file(path);
}

#[test]
fn a_malformed_chunk_is_an_error_not_a_crash() {
    let path = save_path("malformed");
    // A section chunk too short to hold its hash, with a checksum that fits.
    let payload = [1u8, 2, 3, 4];
    let mut bytes = b"rimsave1".to_vec();
    bytes.push(2);
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&payload);
    let check = rim_sim::snapshot::hash_bytes(&[2]) ^ rim_sim::snapshot::hash_bytes(&payload);
    bytes.extend_from_slice(&check.to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();
    assert!(savefile::read(&path).is_err());
    let _ = std::fs::remove_file(path);
}

const BOAR_DEFS: &str = r##"
[[thing]]
id = "hide"
label = "boar hide"
color = "#7a5a3a"
category = "item"
market_value = 3
stack_limit = 20
"##;

/// Remembers a herd in script data, and says so whenever it can see it.
const BOAR_SCRIPT: &str = r#"
rim.every(10, function()
    local herd = rim.get_data("boars:herd")
    if herd == nil then
        rim.set_data("boars:herd", { size = 3 })
    else
        rim.set_data("boars:seen", herd.size)
    end
end)
"#;

#[test]
fn a_removed_mods_data_waits_for_it_and_its_things_are_dropped() {
    let path = save_path("park");
    let with = common::test_mods(
        "park-with",
        &["core", "weather"],
        &[("boars", &[("defs/items.toml", BOAR_DEFS), ("scripts/boars.luau", BOAR_SCRIPT)])],
    );
    let without = common::test_mods("park-without", &["core", "weather"], &[]);

    let (mut sim, mut save) = new_game(&with, &path);
    let hide = sim.world.defs.thing_id("boars:hide").unwrap();
    let c = sim.world.colony_center().unwrap();
    sim.world.place_item(hide, c.offset(-6, -6), 25);
    let mut hashes = BTreeMap::new();
    play(&mut sim, &mut save, 600, &mut hashes);
    save.snapshot(&mut sim).unwrap();
    assert!(sim.world.data.contains_key("boars:herd"));
    drop(save);

    // The player removes the mod.
    let (mut sim, mut save, report) = SaveFile::load(&path, &without, &|_| true).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.new_epoch.is_some());
    assert_eq!(report.dropped, ["dropped 2 × boars:hide"], "25 hides in stacks of 20");
    assert!(sim
        .world
        .ecs
        .query::<&rim_sim::world::Thing>()
        .iter()
        .all(|t| sim.world.defs.thing(t.def).id != "boars:hide"));
    assert!(sim.world.data.contains_key("boars:herd"), "the mod's data is parked in the world");
    play(&mut sim, &mut save, 600, &mut hashes);
    save.snapshot(&mut sim).unwrap();
    drop(save);

    // And puts it back: its data is there for it.
    let (mut sim, mut save, report) = SaveFile::load(&path, &with, &|_| true).unwrap();
    assert!(report.new_epoch.is_some());
    assert!(report.dropped.is_empty(), "{report:?}");
    play(&mut sim, &mut save, 50, &mut hashes);
    assert_eq!(sim.world.data.get("boars:seen"), Some(&rim_sim::data::Data::Int(3)), "the mod reads its herd again");
    drop(save);
    for d in [with, without] {
        let _ = std::fs::remove_dir_all(d);
    }
    let _ = std::fs::remove_file(path);
}

/// A fixture mod at `version`, with `script`.
fn fixture(name: &str, version: &str, script: &str) -> PathBuf {
    let dir = common::test_mods(name, &["core", "weather"], &[("fix", &[("scripts/fix.luau", script)])]);
    let toml = dir.join("fix/mod.toml");
    let text =
        std::fs::read_to_string(&toml).unwrap().replace("version = \"0.1.0\"", &format!("version = \"{version}\""));
    std::fs::write(toml, text).unwrap();
    dir
}

const FIX_V1: &str = r#"
rim.every(10, function()
    if rim.get_data("count") == nil then rim.set_data("count", 5) end
end)
"#;

const FIX_V2: &str = r#"
rim.on_migrate(function(from, data)
    if from == "0.1.0" then
        data.tally = data.count
        data.count = nil
    end
    return data
end)
"#;

#[test]
fn a_mod_upgrades_its_data_when_its_version_changes() {
    let path = save_path("migrate");
    let v1 = fixture("migrate-v1", "0.1.0", FIX_V1);
    let (mut sim, mut save) = new_game(&v1, &path);
    let mut hashes = BTreeMap::new();
    play(&mut sim, &mut save, 60, &mut hashes);
    sim.world.data.insert("weather:untouched".into(), rim_sim::data::Data::Int(1));
    save.snapshot(&mut sim).unwrap();
    assert_eq!(sim.world.data.get("fix:count"), Some(&rim_sim::data::Data::Int(5)));
    drop(save);

    let v2 = fixture("migrate-v2", "0.2.0", FIX_V2);
    let (sim, _, report) = SaveFile::load(&path, &v2, &|_| true).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.new_epoch.is_some());
    assert_eq!(sim.world.data.get("fix:tally"), Some(&rim_sim::data::Data::Int(5)), "migrated");
    assert_eq!(sim.world.data.get("fix:count"), None);
    assert_eq!(sim.world.data.get("weather:untouched"), Some(&rim_sim::data::Data::Int(1)), "only fix's data");
    for d in [v1, v2] {
        let _ = std::fs::remove_dir_all(d);
    }
    let _ = std::fs::remove_file(path);
}

#[test]
fn a_failed_migration_fails_the_load_and_leaves_the_save_alone() {
    let path = save_path("migrate-fail");
    let v1 = fixture("migrate-fail-v1", "0.1.0", FIX_V1);
    let (mut sim, mut save) = new_game(&v1, &path);
    let mut hashes = BTreeMap::new();
    play(&mut sim, &mut save, 60, &mut hashes);
    save.snapshot(&mut sim).unwrap();
    drop(save);
    let before = std::fs::read(&path).unwrap();

    let broken = "rim.on_migrate(function(from, data) error(\"can't read \" .. from) end)\n";
    let v2 = fixture("migrate-fail-v2", "0.2.0", broken);
    let err = SaveFile::load(&path, &v2, &|_| true).err().expect("the load fails");
    assert!(err.contains("mod 'fix'") && err.contains("can't read 0.1.0"), "{err}");
    assert_eq!(std::fs::read(&path).unwrap(), before, "no new epoch was written");
    for d in [v1, v2] {
        let _ = std::fs::remove_dir_all(d);
    }
    let _ = std::fs::remove_file(path);
}

#[test]
fn a_mod_that_comes_back_migrates_from_the_version_its_data_was_written_with() {
    let path = save_path("migrate-return");
    let v1 = fixture("migrate-return-v1", "0.1.0", FIX_V1);
    let gone = common::test_mods("migrate-return-gone", &["core", "weather"], &[]);
    let v2 = fixture("migrate-return-v2", "0.2.0", FIX_V2);
    let (mut sim, mut save) = new_game(&v1, &path);
    let mut hashes = BTreeMap::new();
    play(&mut sim, &mut save, 60, &mut hashes);
    save.snapshot(&mut sim).unwrap();
    drop(save);
    // Played a while without the mod: its data waits, and so does its version.
    let (mut sim, mut save, _) = SaveFile::load(&path, &gone, &|_| true).unwrap();
    play(&mut sim, &mut save, 60, &mut hashes);
    save.snapshot(&mut sim).unwrap();
    drop(save);
    let (sim, _, _) = SaveFile::load(&path, &v2, &|_| true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(sim.world.data.get("fix:tally"), Some(&rim_sim::data::Data::Int(5)), "migrated from 0.1.0");
    for d in [v1, gone, v2] {
        let _ = std::fs::remove_dir_all(d);
    }
    let _ = std::fs::remove_file(path);
}

#[test]
fn a_migration_sees_only_its_data_and_is_registered_at_load() {
    let path = save_path("migrate-pure");
    let v1 = fixture("migrate-pure-v1", "0.1.0", FIX_V1);
    let (mut sim, mut save) = new_game(&v1, &path);
    let mut hashes = BTreeMap::new();
    play(&mut sim, &mut save, 60, &mut hashes);
    save.snapshot(&mut sim).unwrap();
    drop(save);
    let pure = r#"
rim.on_migrate(function(from, data)
    data.had_world = (pcall(rim.random))
    return data
end)
rim.every(5, function()
    rim.set_data("late", (pcall(rim.on_migrate, function(f, d) return d end)))
end)
"#;
    let v2 = fixture("migrate-pure-v2", "0.2.0", pure);
    let (mut sim, mut save, _) = SaveFile::load(&path, &v2, &|_| true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(sim.world.data.get("fix:had_world"), Some(&rim_sim::data::Data::Bool(false)), "no world in a migration");
    play(&mut sim, &mut save, 10, &mut hashes);
    assert_eq!(sim.world.data.get("fix:late"), Some(&rim_sim::data::Data::Bool(false)), "not from a hook");
    drop(save);
    let empty = "rim.on_migrate(function(from, data) return { [\"\"] = 1 } end)\n";
    let v3 = fixture("migrate-pure-v3", "0.3.0", empty);
    let err = SaveFile::load(&path, &v3, &|_| true).err().expect("an empty key fails the load");
    assert!(err.contains("non-empty"), "{err}");
    for d in [v1, v2, v3] {
        let _ = std::fs::remove_dir_all(d);
    }
    let _ = std::fs::remove_file(path);
}
