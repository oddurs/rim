//! The save file (DESIGN.md §7a): the log is the save, snapshots are a
//! cache, and an epoch starts when the code changes.
//!
//! The file only grows. It is a run of chunks, each `kind, length, payload,
//! checksum`; a crash can only cut the last one, and reading stops at the
//! first chunk that doesn't check out. An epoch chunk opens each epoch: the
//! mods and engine it runs under, and its root. Log chunks carry the
//! commands applied since the last one and the snapshot hash at their tick.
//! Snapshot chunks list their sections by hash, and a section's bytes are
//! written once, the first time that hash appears.

use crate::command::Command;
use crate::sim::Sim;
use crate::snapshot::{hash_bytes, Header, Snapshot};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"rimsave1";

const EPOCH: u8 = 1;
const SECTION: u8 = 2;
const SNAPSHOT: u8 = 3;
const LOG: u8 = 4;

/// Where an epoch starts. Every epoch also writes a snapshot at its start,
/// so a load never regenerates the map; the seed is for replays.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Root {
    /// A new game: `Sim::build` with this seed and map size.
    Seed { seed: u64, size: i32 },
    /// The state the previous epoch had reached, loaded under new code.
    Snapshot,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Epoch {
    /// Every mod's (id, version), in load order.
    pub mods: Vec<(String, String)>,
    pub engine: String,
    pub root: Root,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct SnapshotRecord {
    header: Header,
    /// Section name and the hash of its bytes.
    sections: Vec<(String, u64)>,
}

/// The commands applied before `tick` (since the previous log), and the
/// snapshot hash at `tick`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Log {
    pub tick: u64,
    pub commands: Vec<(u64, Command)>,
    pub hash: u64,
}

/// One epoch as read back.
#[derive(Clone, Debug)]
pub struct EpochRead {
    pub epoch: Epoch,
    pub snapshots: Vec<Snapshot>,
    pub logs: Vec<Log>,
}

/// What loading found and did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoadReport {
    /// The tick the game resumes at.
    pub tick: u64,
    /// The tick of the snapshot the load started from.
    pub from_snapshot: u64,
    /// Ticks replayed from the log.
    pub replayed: u64,
    /// Why a new epoch began, if one did.
    pub new_epoch: Option<String>,
    /// The log's hash disagreed here; the game resumes before it.
    pub diverged_at: Option<u64>,
    /// Ticks of log that couldn't be replayed because the code changed.
    pub lost: u64,
    /// What a change of mods took out of the world: "dropped 3 × boars:boar".
    pub dropped: Vec<String>,
    /// Bytes at the end of the file that didn't make a whole chunk.
    pub cut: u64,
    /// Where the file was copied before a damaged tail was cut off.
    pub damaged_copy: Option<PathBuf>,
}

pub struct SaveFile {
    path: PathBuf,
    file: File,
    /// The file's length after the last whole chunk.
    len: u64,
    /// A write failed and couldn't be undone: appending more would bury
    /// good chunks behind a torn one, so nothing more is written.
    broken: bool,
    /// Hashes of the sections already in the file.
    stored: HashSet<u64>,
}

fn chunk(kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 13);
    out.push(kind);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    let check = hash_bytes(&out[..1]) ^ hash_bytes(payload);
    out.extend_from_slice(&check.to_le_bytes());
    out
}

fn msgpack<T: Serialize>(v: &T) -> Vec<u8> {
    rmp_serde::to_vec_named(v).expect("save records are plain data")
}

/// A chunk's kind and payload.
type Chunk<'a> = (u8, &'a [u8]);

/// The whole chunks of a file, and how many bytes they cover.
fn chunks(bytes: &[u8]) -> Result<(Vec<Chunk<'_>>, usize), String> {
    let mut rest = bytes.strip_prefix(MAGIC.as_slice()).ok_or("not a rim save")?;
    let (mut out, mut good) = (Vec::new(), MAGIC.len());
    while rest.len() >= 13 {
        let kind = rest[0];
        let len = u32::from_le_bytes(rest[1..5].try_into().expect("four bytes")) as usize;
        let Some(body) = len.checked_add(13).and_then(|end| rest.get(5..end)) else { break };
        let (payload, check) = body.split_at(len);
        if (hash_bytes(&[kind]) ^ hash_bytes(payload)).to_le_bytes() != check {
            break;
        }
        out.push((kind, payload));
        rest = &rest[5 + len + 8..];
        good += 5 + len + 8;
    }
    Ok((out, good))
}

/// Every epoch in a save file, and how many trailing bytes were cut.
pub fn read(path: &Path) -> Result<(Vec<EpochRead>, u64), String> {
    let mut bytes = Vec::new();
    File::open(path).and_then(|mut f| f.read_to_end(&mut bytes)).map_err(|e| format!("{}: {e}", path.display()))?;
    let (found, good) = chunks(&bytes)?;
    let mut sections: HashMap<u64, Vec<u8>> = HashMap::new();
    let mut epochs: Vec<EpochRead> = Vec::new();
    for (kind, payload) in found {
        match kind {
            EPOCH => {
                let epoch: Epoch = rmp_serde::from_slice(payload).map_err(|e| format!("an epoch: {e}"))?;
                epochs.push(EpochRead { epoch, snapshots: Vec::new(), logs: Vec::new() });
            }
            SECTION => {
                let (h, packed) = payload.split_at_checked(8).ok_or("a section with no hash")?;
                let bytes = zstd::decode_all(packed).map_err(|e| format!("a section: {e}"))?;
                sections.insert(u64::from_le_bytes(h.try_into().expect("eight bytes")), bytes);
            }
            SNAPSHOT => {
                let r: SnapshotRecord = rmp_serde::from_slice(payload).map_err(|e| format!("a snapshot: {e}"))?;
                let mut s = BTreeMap::new();
                for (name, h) in r.sections {
                    let b = sections
                        .get(&h)
                        .ok_or_else(|| format!("the snapshot at tick {} has no section {name}", r.header.tick))?;
                    s.insert(name, b.clone());
                }
                let e = epochs.last_mut().ok_or("a snapshot before any epoch")?;
                e.snapshots.push(Snapshot { header: r.header, sections: s });
            }
            LOG => {
                let l: Log = rmp_serde::from_slice(payload).map_err(|e| format!("a log: {e}"))?;
                epochs.last_mut().ok_or("a log before any epoch")?.logs.push(l);
            }
            k => return Err(format!("unknown chunk kind {k}: a save from a newer version?")),
        }
    }
    Ok((epochs, (bytes.len() - good) as u64))
}

fn lock_of(sim: &Sim) -> Vec<(String, String)> {
    sim.mods.iter().map(|m| (m.id.clone(), m.version.clone())).collect()
}

impl SaveFile {
    /// Start a save for a game just built by `Sim::build`, and start
    /// recording its commands.
    pub fn create(path: &Path, sim: &mut Sim) -> std::io::Result<SaveFile> {
        let mut file = File::create(path)?;
        file.write_all(MAGIC)?;
        let len = MAGIC.len() as u64;
        let mut s = SaveFile { path: path.to_path_buf(), file, len, broken: false, stored: HashSet::new() };
        let root = Root::Seed { seed: sim.world.seed, size: sim.world.map.w };
        s.epoch(sim, root)?;
        Ok(s)
    }

    /// Append one whole chunk, or nothing: a failed write is cut back off.
    fn put(&mut self, kind: u8, payload: &[u8]) -> std::io::Result<()> {
        if self.broken {
            return Err(std::io::Error::other("an earlier write failed; this save takes no more"));
        }
        let c = chunk(kind, payload);
        match self.file.write_all(&c) {
            Ok(()) => {
                self.len += c.len() as u64;
                Ok(())
            }
            Err(e) => {
                let len = self.len;
                if self.file.set_len(len).and_then(|()| self.file.seek(SeekFrom::Start(len))).is_err() {
                    self.broken = true;
                }
                Err(e)
            }
        }
    }

    /// Open an epoch at the game's current state.
    fn epoch(&mut self, sim: &mut Sim, root: Root) -> std::io::Result<()> {
        sim.record();
        sim.clear_applied();
        let e = Epoch { mods: lock_of(sim), engine: env!("CARGO_PKG_VERSION").to_string(), root };
        self.put(EPOCH, &msgpack(&e))?;
        self.write_snapshot(&Snapshot::capture(sim))
    }

    /// Append the commands applied since the last log, and the hash of the
    /// state they led to. Call it as often as losing the tail would hurt; if
    /// it fails, the commands are kept for the next try.
    pub fn log(&mut self, sim: &mut Sim) -> std::io::Result<()> {
        let log = Log { tick: sim.world.tick, commands: sim.applied().to_vec(), hash: Snapshot::capture(sim).hash() };
        self.put(LOG, &msgpack(&log))?;
        sim.clear_applied();
        Ok(())
    }

    /// Log, then write a snapshot: its record, and any section this file
    /// doesn't hold yet.
    pub fn snapshot(&mut self, sim: &mut Sim) -> std::io::Result<()> {
        self.log(sim)?;
        self.write_snapshot(&Snapshot::capture(sim))
    }

    fn write_snapshot(&mut self, snap: &Snapshot) -> std::io::Result<()> {
        let mut record = SnapshotRecord { header: snap.header.clone(), sections: Vec::new() };
        for (name, bytes) in &snap.sections {
            let h = hash_bytes(bytes);
            if !self.stored.contains(&h) {
                let mut payload = h.to_le_bytes().to_vec();
                payload.extend(zstd::encode_all(bytes.as_slice(), 3)?);
                self.put(SECTION, &payload)?;
                self.stored.insert(h);
            }
            record.sections.push((name.clone(), h));
        }
        self.put(SNAPSHOT, &msgpack(&record))
    }

    /// Push what's been written through to the disk.
    pub fn sync(&mut self) -> std::io::Result<()> {
        self.file.sync_data()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Open a save and bring its game back: the newest snapshot of the last
    /// epoch, then its log, checking the hash at every log chunk. A torn
    /// tail is cut off. A new epoch begins when the mods or the engine
    /// differ from the save's (the log only replays under the code that
    /// wrote it), or when the log stops agreeing with itself.
    pub fn load(
        path: &Path,
        mods_dir: &Path,
        enabled: &dyn Fn(&str) -> bool,
    ) -> Result<(Sim, SaveFile, LoadReport), String> {
        let (epochs, cut) = read(path)?;
        let last = epochs.last().ok_or("a save with no epoch")?;
        let snap = last.snapshots.last().ok_or("an epoch with no snapshot")?;
        let mut report = LoadReport { cut, from_snapshot: snap.header.tick, ..Default::default() };
        let (mut sim, dropped) = snap.restore_noting(mods_dir, enabled)?;
        report.dropped = dropped;
        let start = sim.world.tick;
        let same_code = lock_of(&sim) == last.epoch.mods && last.epoch.engine == env!("CARGO_PKG_VERSION");
        if !same_code {
            report.lost = last.logs.iter().map(|l| l.tick).max().unwrap_or(start).saturating_sub(start);
        } else if let Err(at) = replay(&mut sim, &last.logs, None) {
            // Start over, and stop at the last tick the log vouched for.
            report.diverged_at = Some(at);
            let good = last.logs.iter().map(|l| l.tick).filter(|&t| t < at).max().unwrap_or(start).max(start);
            sim = snap.restore(mods_dir, enabled)?;
            replay(&mut sim, &last.logs, Some(good)).map_err(|t| format!("the log diverged again at tick {t}"))?;
        }
        report.replayed = sim.world.tick - start;
        report.tick = sim.world.tick;

        // Written at its end, but not opened to append: Windows won't cut a
        // file opened only for appending.
        let mut file = OpenOptions::new().write(true).open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let len = file.metadata().map_err(|e| e.to_string())?.len() - cut;
        if cut > 0 {
            // Usually a write the process didn't live to finish. It could be
            // damage in the middle, with good chunks after it: keep a copy.
            let copy = path.with_extension("damaged");
            std::fs::copy(path, &copy).map_err(|e| format!("{}: {e}", copy.display()))?;
            file.set_len(len).map_err(|e| e.to_string())?;
            report.damaged_copy = Some(copy);
        }
        file.seek(SeekFrom::End(0)).map_err(|e| e.to_string())?;
        let stored = epochs.iter().flat_map(|e| &e.snapshots).flat_map(|s| s.sections.values()).map(|b| hash_bytes(b));
        let mut save = SaveFile { path: path.to_path_buf(), file, len, broken: false, stored: stored.collect() };

        report.new_epoch = if !same_code {
            Some("the mods or the engine changed".to_string())
        } else {
            report.diverged_at.map(|t| format!("the log stopped replaying at tick {t}"))
        };
        if report.new_epoch.is_some() {
            save.epoch(&mut sim, Root::Snapshot).map_err(|e| e.to_string())?;
        } else {
            sim.record();
        }
        Ok((sim, save, report))
    }
}

/// Run `sim` through the logs after its tick, up to `until` (or the last
/// log), each command at its tick. Err is the first log whose hash disagrees.
fn replay(sim: &mut Sim, logs: &[Log], until: Option<u64>) -> Result<(), u64> {
    let from = sim.world.tick;
    for log in logs.iter().filter(|l| l.tick > from && until.is_none_or(|u| l.tick <= u)) {
        let mut cmds = log.commands.iter().filter(|c| c.0 >= from).peekable();
        while sim.world.tick < log.tick {
            while let Some((_, c)) = cmds.next_if(|c| c.0 == sim.world.tick) {
                sim.push(c.clone());
            }
            sim.step();
        }
        if Snapshot::capture(sim).hash() != log.hash {
            return Err(log.tick);
        }
    }
    Ok(())
}
