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
/// hash of each snapshot section at `tick`, so a replay that disagrees can
/// say where.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Log {
    pub tick: u64,
    pub commands: Vec<(u64, Command)>,
    pub hashes: BTreeMap<String, u64>,
}

impl Log {
    /// The commands applied since the last log, and the state they led to.
    fn of(sim: &Sim) -> Log {
        Log { tick: sim.world.tick, commands: sim.applied().to_vec(), hashes: section_hashes(sim) }
    }
}

fn section_hashes(sim: &Sim) -> BTreeMap<String, u64> {
    Snapshot::capture(sim).sections.iter().map(|(n, b)| (n.clone(), hash_bytes(b))).collect()
}

/// The sections whose hashes differ between two logs' worth of hashes.
fn differing(a: &BTreeMap<String, u64>, b: &BTreeMap<String, u64>) -> Vec<String> {
    let names: std::collections::BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    names.into_iter().filter(|n| a.get(*n) != b.get(*n)).cloned().collect()
}

/// What replaying an epoch found.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayReport {
    pub epoch: usize,
    /// Where it started: the seed, or the tick of the epoch's first snapshot.
    pub root: Root,
    pub from_tick: u64,
    /// Log ticks whose hashes matched.
    pub checked: Vec<u64>,
    /// The first log that disagreed, and the sections that differ there.
    pub diverged: Option<(u64, Vec<String>)>,
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

/// Write epochs as a new file, the inverse of `read`: each epoch, then its
/// logs and snapshots in tick order, a log before the snapshot at its tick.
/// It's written beside `path` and renamed over it, so a failure leaves
/// whatever was there.
pub fn write(path: &Path, epochs: &[EpochRead]) -> std::io::Result<()> {
    let mut name = path.as_os_str().to_owned();
    name.push(".new");
    let tmp = PathBuf::from(name);
    let written = (|| {
        let mut file = File::create(&tmp)?;
        file.write_all(MAGIC)?;
        let len = MAGIC.len() as u64;
        let mut s = SaveFile { path: path.to_path_buf(), file, len, broken: false, stored: HashSet::new() };
        for e in epochs {
            s.put(EPOCH, &msgpack(&e.epoch))?;
            let (mut snaps, mut logs) = (e.snapshots.iter().peekable(), e.logs.iter().peekable());
            loop {
                if let Some(log) = logs.next_if(|l| snaps.peek().is_none_or(|s| l.tick <= s.header.tick)) {
                    s.put(LOG, &msgpack(log))?;
                } else if let Some(snap) = snaps.next() {
                    s.write_snapshot(snap)?;
                } else {
                    break;
                }
            }
        }
        s.sync()
    })();
    match written {
        Ok(()) => std::fs::rename(&tmp, path),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// A save as a list of saves shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    /// How far the colony got: its newest log or snapshot.
    pub tick: u64,
    /// The living colonists at the newest snapshot, the founder first.
    pub colonists: Vec<String>,
}

/// What a save holds, read without loading the game.
pub fn summary(path: &Path) -> Result<Summary, String> {
    let (epochs, _) = read(path)?;
    let e = epochs.last().ok_or("a save with no epoch")?;
    let snap = e.snapshots.last().ok_or("an epoch with no snapshot")?;
    let tick = e.logs.last().map_or(0, |l| l.tick).max(snap.header.tick);
    Ok(Summary { tick, colonists: snap.colonists()? })
}

fn lock_of(sim: &Sim) -> Vec<(String, String)> {
    sim.mods.iter().map(|m| (m.id.clone(), m.version.clone())).collect()
}

impl SaveFile {
    /// Start a save for a game, and start recording its commands. A game
    /// that hasn't run yet is rooted at its seed, so it replays from there;
    /// one that has is rooted at its snapshot.
    pub fn create(path: &Path, sim: &mut Sim) -> std::io::Result<SaveFile> {
        let mut s = SaveFile::blank(path)?;
        let root = match sim.world.tick {
            0 => Root::Seed { seed: sim.world.seed, size: sim.world.map.w },
            _ => Root::Snapshot,
        };
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

    /// A new file with nothing in it yet.
    fn blank(path: &Path) -> std::io::Result<SaveFile> {
        let mut file = File::create(path)?;
        file.write_all(MAGIC)?;
        let len = MAGIC.len() as u64;
        Ok(SaveFile { path: path.to_path_buf(), file, len, broken: false, stored: HashSet::new() })
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
        self.write_log(&Log::of(sim))?;
        sim.clear_applied();
        Ok(())
    }

    fn write_log(&mut self, log: &Log) -> std::io::Result<()> {
        self.put(LOG, &msgpack(log))
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
        } else if let Err((at, _)) = replay_logs(&mut sim, &last.logs, None, &mut |_| {}) {
            // Start over, and stop at the last tick the log vouched for.
            report.diverged_at = Some(at);
            let good = last.logs.iter().map(|l| l.tick).filter(|&t| t < at).max().unwrap_or(start).max(start);
            sim = snap.restore(mods_dir, enabled)?;
            replay_logs(&mut sim, &last.logs, Some(good), &mut |_| {})
                .map_err(|(t, _)| format!("the log diverged again at tick {t}"))?;
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
/// log), each command at its tick, calling `checked` with each log tick that
/// agreed. Err is the first log that disagreed, and the sections that differ.
fn replay_logs(
    sim: &mut Sim,
    logs: &[Log],
    until: Option<u64>,
    checked: &mut dyn FnMut(u64),
) -> Result<(), (u64, Vec<String>)> {
    let from = sim.world.tick;
    for log in logs.iter().filter(|l| l.tick > from && until.is_none_or(|u| l.tick <= u)) {
        let mut cmds = log.commands.iter().filter(|c| c.0 >= from).peekable();
        while sim.world.tick < log.tick {
            while let Some((_, c)) = cmds.next_if(|c| c.0 == sim.world.tick) {
                sim.push(c.clone());
            }
            sim.step();
        }
        let diff = differing(&section_hashes(sim), &log.hashes);
        if !diff.is_empty() {
            return Err((log.tick, diff));
        }
        checked(log.tick);
    }
    Ok(())
}

/// Replay one epoch of a save from its root, the way a bug report is
/// reproduced: a new game from the seed (or the epoch's first snapshot),
/// then every command at its tick, checking each log's hashes. The mods
/// must be the ones the epoch ran under. `epoch` defaults to the last.
pub fn replay(path: &Path, mods_dir: &Path, epoch: Option<usize>) -> Result<ReplayReport, String> {
    let (epochs, _) = read(path)?;
    let n = epoch.unwrap_or(epochs.len().saturating_sub(1));
    let e = epochs.get(n).ok_or_else(|| format!("the save has {} epochs; there's no epoch {n}", epochs.len()))?;
    if e.epoch.engine != env!("CARGO_PKG_VERSION") {
        return Err(format!(
            "epoch {n} ran under engine {}; this is {}, and a log only replays under the code that wrote it",
            e.epoch.engine,
            env!("CARGO_PKG_VERSION")
        ));
    }
    let ids: Vec<&str> = e.epoch.mods.iter().map(|m| m.0.as_str()).collect();
    let enabled = |m: &str| ids.contains(&m);
    let mut sim = match e.epoch.root {
        Root::Seed { seed, size } => Sim::build(mods_dir, seed, &enabled, size)?,
        Root::Snapshot => e.snapshots.first().ok_or("an epoch with no snapshot")?.restore(mods_dir, &enabled)?,
    };
    if lock_of(&sim) != e.epoch.mods {
        return Err(format!("epoch {n} ran under mods {:?}; installed: {:?}", e.epoch.mods, lock_of(&sim)));
    }
    let from_tick = sim.world.tick;
    // A new game from the seed must be the one the save began with.
    if let Some(first) = e.snapshots.first().filter(|s| s.header.tick == from_tick) {
        let saved = first.sections.iter().map(|(n, b)| (n.clone(), hash_bytes(b))).collect();
        let diff = differing(&section_hashes(&sim), &saved);
        if !diff.is_empty() {
            return Ok(ReplayReport {
                epoch: n,
                root: e.epoch.root.clone(),
                from_tick,
                checked: Vec::new(),
                diverged: Some((from_tick, diff)),
            });
        }
    }
    let mut checked = Vec::new();
    let diverged = replay_logs(&mut sim, &e.logs, None, &mut |t| checked.push(t)).err();
    Ok(ReplayReport { epoch: n, root: e.epoch.root.clone(), from_tick, checked, diverged })
}

/// Rewrite a save without the snapshots nothing needs: each epoch keeps the
/// snapshot it began with (replays start there) and the last epoch keeps its
/// newest `keep` as well. Every log stays, so nothing can be lost; only the
/// cache gets smaller. Written beside the file, then renamed over it.
pub fn compact(path: &Path, keep: usize) -> Result<(), String> {
    let (epochs, cut) = read(path)?;
    if cut > 0 {
        // Only a load cuts a damaged tail, and it keeps a copy first.
        return Err(format!("{} has a damaged end: load it before compacting it", path.display()));
    }
    let tmp = path.with_extension("compacting");
    let err = |e: std::io::Error| format!("{}: {e}", tmp.display());
    let mut out = SaveFile::blank(&tmp).map_err(err)?;
    for (i, e) in epochs.iter().enumerate() {
        out.put(EPOCH, &msgpack(&e.epoch)).map_err(err)?;
        let newest = if i + 1 == epochs.len() { keep } else { 0 };
        let kept: Vec<&Snapshot> = e
            .snapshots
            .iter()
            .enumerate()
            .filter(|(k, _)| *k == 0 || *k + newest >= e.snapshots.len())
            .map(|(_, s)| s)
            .collect();
        // Back in the order they were written: the root first, then logs and
        // snapshots by tick, each snapshot after the log that reached it.
        let mut items: Vec<(u64, u8, Option<&Snapshot>, Option<&Log>)> = Vec::new();
        for (k, s) in kept.iter().enumerate() {
            items.push((s.header.tick, if k == 0 { 0 } else { 2 }, Some(s), None));
        }
        items.extend(e.logs.iter().map(|l| (l.tick, 1, None, Some(l))));
        items.sort_by_key(|x| (x.0, x.1));
        for (_, _, snap, log) in items {
            match (snap, log) {
                (Some(s), _) => out.write_snapshot(s).map_err(err)?,
                (_, Some(l)) => out.write_log(l).map_err(err)?,
                _ => unreachable!("each item is one or the other"),
            }
        }
    }
    out.sync().map_err(err)?;
    drop(out);
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

enum Job {
    Log(Log),
    Snapshot(Log, Snapshot),
}

/// A save file on a thread of its own: the game hands it logs and snapshots
/// (capturing one takes under a millisecond) and carries on, while the
/// compressing and writing happen elsewhere.
pub struct Writer {
    tx: Option<std::sync::mpsc::Sender<Job>>,
    thread: Option<std::thread::JoinHandle<Result<(), String>>>,
    /// The last write that failed, for the player to hear about.
    failed: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl Writer {
    pub fn spawn(mut save: SaveFile) -> Writer {
        let (tx, rx) = std::sync::mpsc::channel::<Job>();
        let failed = std::sync::Arc::new(std::sync::Mutex::new(None));
        let report = failed.clone();
        let thread = std::thread::spawn(move || {
            // Commands whose log didn't make it to disk ride in the next one,
            // so the log never skips a command the state went on to reflect.
            let mut carried: Vec<(u64, Command)> = Vec::new();
            let mut last_err = None;
            for job in rx {
                let (mut log, snap) = match job {
                    Job::Log(l) => (l, None),
                    Job::Snapshot(l, s) => (l, Some(s)),
                };
                log.commands.splice(0..0, carried.drain(..));
                let mut r = save.write_log(&log);
                if r.is_err() {
                    carried = log.commands;
                }
                // The snapshot is worth writing even if the log wasn't.
                if let Some(s) = snap {
                    r = r.and(save.write_snapshot(&s));
                }
                // Keep going: a later write may succeed, and `put` refuses to
                // append once the file can't be kept whole.
                if let Err(e) = r {
                    let e = format!("saving to {} failed: {e}", save.path().display());
                    eprintln!("rim: {e}");
                    *report.lock().unwrap_or_else(|p| p.into_inner()) = Some(e.clone());
                    last_err = Some(e);
                }
            }
            save.sync().map_err(|e| e.to_string())?;
            last_err.map_or(Ok(()), Err)
        });
        Writer { tx: Some(tx), thread: Some(thread), failed }
    }

    /// The latest save failure since the last call, if any.
    pub fn take_error(&self) -> Option<String> {
        self.failed.lock().unwrap_or_else(|p| p.into_inner()).take()
    }

    /// Log the commands applied since the last log.
    pub fn log(&self, sim: &mut Sim) {
        self.send(Job::Log(Log::of(sim)));
        sim.clear_applied();
    }

    /// Log, and take a snapshot.
    pub fn snapshot(&self, sim: &mut Sim) {
        self.send(Job::Snapshot(Log::of(sim), Snapshot::capture(sim)));
        sim.clear_applied();
    }

    fn send(&self, job: Job) {
        if let Some(tx) = &self.tx {
            // The thread only stops early by panicking.
            if tx.send(job).is_err() {
                *self.failed.lock().unwrap_or_else(|p| p.into_inner()) = Some("the save thread stopped".into());
            }
        }
    }

    /// Write everything handed over so far, and wait for the disk.
    pub fn finish(mut self) -> Result<(), String> {
        self.stop()
    }

    fn stop(&mut self) -> Result<(), String> {
        drop(self.tx.take());
        match self.thread.take() {
            Some(t) => t.join().map_err(|_| "the save thread panicked".to_string())?,
            None => Ok(()),
        }
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            eprintln!("rim: {e}");
        }
    }
}
