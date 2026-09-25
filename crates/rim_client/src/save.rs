//! The game is always saved (DESIGN.md §7a): every colony has a save file,
//! its log is appended as it plays, and snapshots are taken on a cadence and
//! on quit. The sim thread only captures; a writer thread does the rest.

use rim_sim::savefile::{self, SaveFile, Writer};
use rim_sim::{Sim, TICKS_PER_DAY};
use std::path::{Path, PathBuf};

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
