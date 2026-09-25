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
