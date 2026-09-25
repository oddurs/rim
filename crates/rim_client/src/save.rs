//! The game is always saved (DESIGN.md §7a): every colony has a save file,
//! its log is appended as it plays, and snapshots are taken on a cadence and
//! on quit. The sim thread only captures; a writer thread does the rest.

use rim_sim::savefile::{self, SaveFile, Writer};
use rim_sim::{Sim, TICKS_PER_DAY};
use rim_ui::view::SaveView;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// How often the log is appended: ten seconds at normal speed. A crash
/// loses at most this much.
const LOG_EVERY: u64 = 600;
/// How often a snapshot is taken, so a load replays at most this much.
const SNAPSHOT_EVERY: u64 = TICKS_PER_DAY / 4;
/// Snapshots a save keeps besides its epochs' first ones.
const KEEP: usize = 4;

/// Where a game comes from.
pub enum Start {
    New,
    Load(PathBuf),
}

/// `--load FILE`, `--continue` (the newest save), or a new colony.
pub fn start_of(args: &[String]) -> Result<Start, String> {
    if let Some(i) = args.iter().position(|a| a == "--load") {
        let file = args.get(i + 1).filter(|a| !a.starts_with("--")).ok_or("--load takes a save file")?;
        return Ok(Start::Load(PathBuf::from(file)));
    }
    if args.iter().any(|a| a == "--continue") {
        let dir = dir().ok_or("no saves folder on this system")?;
        return newest(&dir).map(Start::Load).ok_or_else(|| format!("no saves in {}", dir.display()));
    }
    Ok(Start::New)
}

/// The player's saves, beside their UI layout.
fn dir() -> Option<PathBuf> {
    crate::player_file("saves")
}

fn newest(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "rim"))
        .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())
        .map(|e| e.path())
}

/// Every save in the saves folder, newest first, as the title screen lists
/// them. One that can't be read is listed with why.
pub fn list() -> Vec<SaveView> {
    dir().map_or(Vec::new(), |d| list_in(&d))
}

fn list_in(dir: &Path) -> Vec<SaveView> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut found: Vec<(SystemTime, PathBuf)> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "rim"))
        .map(|p| (p.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH), p))
        .collect();
    found.sort_by(|a, b| b.cmp(a));
    let now = SystemTime::now();
    found
        .into_iter()
        .map(|(played, path)| {
            let summary = savefile::summary(&path);
            SaveView {
                path: path.to_string_lossy().into_owned(),
                file: path.file_name().map_or(String::new(), |f| f.to_string_lossy().into_owned()),
                day: summary.as_ref().map_or(0, |s| s.tick / TICKS_PER_DAY + 1),
                colonists: summary.as_ref().map_or(Vec::new(), |s| s.colonists.clone()),
                age: now.duration_since(played).unwrap_or_default().as_secs_f64(),
                error: summary.err(),
            }
        })
        .collect()
}

/// Build or load the game, with its writer. The strings are what a load
/// found worth telling the player: mods that changed, things dropped.
pub fn open(mods: &Path, seed: u64, start: Start) -> Result<(Sim, Option<Writer>, Vec<String>), String> {
    match start {
        Start::New => {
            let mut sim = Sim::new(mods, seed)?;
            let Some(dir) = dir() else {
                return Ok((sim, None, vec!["no saves folder on this system: this colony won't be saved".into()]));
            };
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let path = (1..).map(|n| dir.join(name(seed, n))).find(|p| !p.exists()).expect("a free name");
            let save = SaveFile::create(&path, &mut sim).map_err(|e| format!("{}: {e}", path.display()))?;
            eprintln!("rim: saving to {}", path.display());
            Ok((sim, Some(Writer::spawn(save)), Vec::new()))
        }
        Start::Load(path) => {
            let (epochs, cut) = savefile::read(&path)?;
            // A damaged file is the load's to cut, keeping a copy.
            if cut == 0 && epochs.last().is_some_and(|e| e.snapshots.len() > KEEP + 1) {
                savefile::compact(&path, KEEP)?;
            }
            let (sim, save, r) = SaveFile::load(&path, mods, &|_| true)?;
            eprintln!("rim: loaded {} at tick {} ({} ticks replayed)", path.display(), r.tick, r.replayed);
            let mut notes: Vec<String> = r.dropped.clone();
            if let Some(why) = &r.new_epoch {
                notes.push(format!("{why}: a new epoch begins at tick {}", r.tick));
            }
            if r.lost > 0 {
                notes.push(format!("{} ticks after the last snapshot were made under other mods and are gone", r.lost));
            }
            if let Some(copy) = &r.damaged_copy {
                notes.push(format!("the save's end was damaged; the file as it was is at {}", copy.display()));
            }
            Ok((sim, Some(Writer::spawn(save)), notes))
        }
    }
}

fn name(seed: u64, n: u32) -> String {
    match n {
        1 => format!("colony-{seed}.rim"),
        n => format!("colony-{seed}-{n}.rim"),
    }
}

/// After every sim step: log, or snapshot, when it's time. A write that
/// failed is told to the player with the load warnings.
pub fn after_step(w: &Writer, sim: &mut Sim) {
    let t = sim.world.tick;
    if t.is_multiple_of(SNAPSHOT_EVERY) {
        w.snapshot(sim);
    } else if t.is_multiple_of(LOG_EVERY) {
        w.log(sim);
    } else {
        return;
    }
    if let Some(e) = w.take_error() {
        sim.warnings.push(format!("this colony isn't being saved: {e}"));
    }
}

/// The player is leaving: a last snapshot, so the next load replays nothing
/// and survives a change of mods, then wait for the disk.
pub fn close(w: Writer, sim: &mut Sim) {
    w.snapshot(sim);
    if let Err(e) = w.finish() {
        eprintln!("rim: the last save didn't finish: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_are_listed_newest_first_and_a_broken_one_says_why() {
        let dir = std::env::temp_dir().join(format!("rim-saves-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
        for (file, ticks) in [("colony-1.rim", 600), ("colony-2.rim", 2 * TICKS_PER_DAY)] {
            let mut sim = Sim::new(&mods, 1).unwrap();
            let mut save = SaveFile::create(&dir.join(file), &mut sim).unwrap();
            for _ in 0..ticks {
                sim.step();
            }
            save.log(&mut sim).unwrap();
        }
        std::fs::write(dir.join("colony-3.rim"), "not a save").unwrap();

        let saves = list_in(&dir);
        let files: Vec<&str> = saves.iter().map(|s| s.file.as_str()).collect();
        assert_eq!(files, ["colony-3.rim", "colony-2.rim", "colony-1.rim"], "newest first");
        assert_eq!(saves[0].error.as_deref(), Some("not a rim save"));
        assert_eq!((saves[1].day, saves[2].day), (3, 1));
        assert!(!saves[1].colonists.is_empty() && saves[1].error.is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
