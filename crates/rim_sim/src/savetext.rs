//! Saves as text, for people (DESIGN.md §7a): `unpack` writes a save as a
//! directory of JSON, `pack` turns it back, and `diff` compares two saves.
//!
//! ```text
//! dir/epoch-0/epoch.json          the mods and engine it ran under, its root
//! dir/epoch-0/log.json            its commands, and the state hash at each log
//! dir/epoch-0/tick-0/             a snapshot: snapshot.json (its header and
//!                                 hash), and one file per section:
//!                                 engine/pawn.json, weather/data.json, ...
//! ```
//!
//! A def is named by its qualified id ("core:wall") and the map is drawn,
//! one line per row. Each section goes through its Rust type both ways, so
//! a snapshot nobody touched packs back to the same bytes. One whose hash
//! no longer matches was edited: it becomes the root of a new epoch, and
//! the history it came from stays behind it.

use crate::data::Data;
use crate::defs::DefId;
use crate::field::SavedFields;
use crate::savefile::{self, Epoch, EpochRead, Log, Root};
use crate::snapshot::{DefsSection, Header, ScriptsSection, Snapshot, WorldSection};
use crate::world::{Blueprint, Designated, MadeOf, Owner, Pawn, Regrow, Thing, Work};
use hecs::Entity;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Where each section names a def, and which kind: a path of field names,
/// `*` for every element, and tuple positions. A command is `[tick, command]`.
const DEF_REFS: &[(&str, &str, &str)] = &[
    ("engine:pawn", "*.1.def", "creature"),
    ("engine:pawn", "*.1.needs.*.0", "need"),
    ("engine:pawn", "*.1.carry.0", "thing"),
    ("engine:pawn", "*.1.job.Comfort.need", "need"),
    ("engine:thing", "*.1.def", "thing"),
    ("engine:blueprint", "*.1.cost.*.0", "thing"),
    ("engine:made_of", "*.1", "thing"),
    ("engine:designated", "*.1", "designation"),
    ("engine:world", "events.*.PawnJoined.def", "creature"),
    ("engine:world", "events.*.PawnDied.def", "creature"),
    ("engine:world", "events.*.PawnLeft.def", "creature"),
    ("engine:world", "events.*.BuildingComplete.def", "thing"),
    ("log", "*.commands.*.1.Designate.designation", "designation"),
    ("log", "*.commands.*.1.Build.thing", "thing"),
    ("log", "*.commands.*.1.Build.stuff", "thing"),
];

/// A section's bytes as JSON and back, through the type it holds.
struct Codec {
    to: fn(&[u8]) -> Result<Value, String>,
    from: fn(Value) -> Result<Vec<u8>, String>,
}

fn codec<T: Serialize + DeserializeOwned>() -> Codec {
    Codec {
        to: |b| {
            let v: T = rmp_serde::from_slice(b).map_err(|e| e.to_string())?;
            serde_json::to_value(v).map_err(|e| e.to_string())
        },
        from: |v| {
            let t: T = serde_json::from_value(v).map_err(|e| e.to_string())?;
            rmp_serde::to_vec_named(&t).map_err(|e| e.to_string())
        },
    }
}

fn codec_of(section: &str) -> Result<Codec, String> {
    Ok(match section {
        "engine:defs" => codec::<DefsSection>(),
        "engine:world" => codec::<WorldSection>(),
        "engine:map" => codec::<Vec<DefId>>(),
        "engine:fields" => codec::<SavedFields>(),
        "engine:scripts" => codec::<ScriptsSection>(),
        "engine:pawn" => codec::<Vec<(Entity, Pawn)>>(),
        "engine:thing" => codec::<Vec<(Entity, Thing)>>(),
        "engine:blueprint" => codec::<Vec<(Entity, Blueprint)>>(),
        "engine:made_of" => codec::<Vec<(Entity, MadeOf)>>(),
        "engine:owner" => codec::<Vec<(Entity, Owner)>>(),
        "engine:designated" => codec::<Vec<(Entity, Designated)>>(),
        "engine:regrow" => codec::<Vec<(Entity, Regrow)>>(),
        "engine:work" => codec::<Vec<(Entity, Work)>>(),
        "log" => codec::<Vec<Log>>(),
        s if s.ends_with(":data") => codec::<BTreeMap<String, Data>>(),
        s => return Err(format!("{s}: this version has no text form for it")),
    })
}

/// Call `f` on every value at `path` ("a.*.1").
fn each_at(v: &mut Value, path: &[&str], f: &mut dyn FnMut(&mut Value) -> Result<(), String>) -> Result<(), String> {
    let Some((head, rest)) = path.split_first() else { return f(v) };
    let next = match (*head, v) {
        ("*", Value::Array(a)) => return a.iter_mut().try_for_each(|x| each_at(x, rest, f)),
        (i, Value::Array(a)) => i.parse().ok().and_then(|i: usize| a.get_mut(i)),
        (k, Value::Object(o)) => o.get_mut(k),
        // A None, or another variant.
        _ => None,
    };
    next.map_or(Ok(()), |x| each_at(x, rest, f))
}

/// Def indices in a section's JSON become qualified ids, or back.
fn name_defs(section: &str, v: &mut Value, defs: &DefsSection, to_names: bool) -> Result<(), String> {
    for (_, path, kind) in DEF_REFS.iter().filter(|r| r.0 == section) {
        let ids = defs.get(*kind).map_or(&[][..], |v| v.as_slice());
        let path: Vec<&str> = path.split('.').collect();
        each_at(v, &path, &mut |x| {
            match x {
                Value::Number(n) if to_names => {
                    if let Some(id) = n.as_u64().and_then(|i| ids.get(i as usize)) {
                        *x = Value::String(id.clone());
                    }
                }
                Value::String(s) if !to_names => {
                    let i = ids.iter().position(|id| id == s).ok_or_else(|| format!("{section}: no {kind} {s}"))?;
                    *x = Value::from(i);
                }
                _ => {}
            }
            Ok(())
        })?;
    }
    Ok(())
}

/// Characters for the map picture, one per terrain.
fn glyph(i: usize) -> Option<char> {
    const ASCII: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    match ASCII.get(i) {
        Some(&c) => Some(c as char),
        // Then the Latin letters past ASCII.
        None => char::from_u32(0xc0 + (i - ASCII.len()) as u32).filter(|c| c.is_alphabetic()),
    }
}

/// The map as rows of glyphs, and a legend of which terrain each is.
fn draw_map(terrain: &[DefId], width: usize, defs: &DefsSection) -> Result<Value, String> {
    let names = defs.get("terrain").map_or(&[][..], |v| v.as_slice());
    let glyphs: Vec<char> = (0..names.len()).map(glyph).collect::<Option<_>>().ok_or("too many terrains to draw")?;
    let rows: Vec<Value> = terrain
        .chunks(width.max(1))
        .map(|row| row.iter().map(|&t| glyphs.get(t as usize).copied().unwrap_or('?')).collect::<String>().into())
        .collect();
    let legend: Map<String, Value> = glyphs.iter().zip(names).map(|(g, n)| (g.to_string(), n.clone().into())).collect();
    Ok(serde_json::json!({ "legend": legend, "rows": rows }))
}

fn read_map(v: Value, defs: &DefsSection) -> Result<Vec<DefId>, String> {
    #[derive(Deserialize)]
    struct Picture {
        legend: BTreeMap<char, String>,
        rows: Vec<String>,
    }
    let p: Picture = serde_json::from_value(v).map_err(|e| format!("engine:map: {e}"))?;
    let names = defs.get("terrain").map_or(&[][..], |v| v.as_slice());
    let mut out = Vec::new();
    for (y, row) in p.rows.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            let name = p.legend.get(&c).ok_or_else(|| format!("engine:map: {c} at ({x}, {y}) isn't in the legend"))?;
            let i = names.iter().position(|n| n == name).ok_or_else(|| format!("engine:map: no terrain {name}"))?;
            out.push(i as DefId);
        }
    }
    Ok(out)
}

/// The def table a snapshot's def indices point into.
fn defs_of(snap: Option<&Snapshot>) -> Result<DefsSection, String> {
    match snap.and_then(|s| s.sections.get("engine:defs")) {
        Some(b) => rmp_serde::from_slice(b).map_err(|e| format!("engine:defs: {e}")),
        None => Ok(DefsSection::new()),
    }
}

/// A snapshot's sections as JSON, by name.
fn snapshot_text(snap: &Snapshot) -> Result<BTreeMap<String, Value>, String> {
    let mut out = BTreeMap::new();
    for (name, bytes) in &snap.sections {
        out.insert(name.clone(), (codec_of(name)?.to)(bytes).map_err(|e| format!("{name}: {e}"))?);
    }
    let defs = defs_of(Some(snap))?;
    if let Some(map) = out.remove("engine:map") {
        let width = out.get("engine:world").and_then(|w| w["width"].as_u64()).ok_or("the world has no width")?;
        let terrain: Vec<DefId> = serde_json::from_value(map).map_err(|e| e.to_string())?;
        out.insert("engine:map".into(), draw_map(&terrain, width as usize, &defs)?);
    }
    for (name, v) in &mut out {
        name_defs(name, v, &defs, true)?;
    }
    Ok(out)
}

/// JSON on one line, spaced for reading.
fn flat(v: &Value) -> String {
    match v {
        Value::Array(a) => format!("[{}]", a.iter().map(flat).collect::<Vec<_>>().join(", ")),
        Value::Object(o) => {
            let fields: Vec<String> =
                o.iter().map(|(k, x)| format!("{}: {}", Value::from(k.as_str()), flat(x))).collect();
            format!("{{{}}}", fields.join(", "))
        }
        v => v.to_string(),
    }
}

/// JSON laid out for reading and diffing: what fits on a line is one line,
/// a list of numbers fills its lines like text, and any other container
/// that doesn't fit has one child per line.
fn layout(v: &Value, indent: usize, out: &mut String) {
    const WIDTH: usize = 100;
    let line = flat(v);
    let children: Vec<(Option<&String>, &Value)> = match v {
        Value::Array(a) => a.iter().map(|x| (None, x)).collect(),
        Value::Object(o) => o.iter().map(|(k, x)| (Some(k), x)).collect(),
        _ => Vec::new(),
    };
    if children.is_empty() || indent + line.len() <= WIDTH {
        return out.push_str(&line);
    }
    let pad = " ".repeat(indent + 2);
    let (open, close) = if v.is_array() { ('[', ']') } else { ('{', '}') };
    out.push(open);
    if v.is_array() && children.iter().all(|(_, x)| x.is_number()) {
        let mut col = WIDTH;
        for (i, (_, x)) in children.iter().enumerate() {
            let item = format!("{x}{}", if i + 1 < children.len() { "," } else { "" });
            if col + 1 + item.len() > WIDTH {
                out.push('\n');
                out.push_str(&pad);
                col = pad.len();
            } else {
                out.push(' ');
                col += 1;
            }
            out.push_str(&item);
            col += item.len();
        }
    } else {
        for (i, (k, x)) in children.iter().enumerate() {
            out.push('\n');
            out.push_str(&pad);
            if let Some(k) = k {
                out.push_str(&Value::from(k.as_str()).to_string());
                out.push_str(": ");
            }
            layout(x, indent + 2, out);
            if i + 1 < children.len() {
                out.push(',');
            }
        }
    }
    out.push('\n');
    out.push_str(&" ".repeat(indent));
    out.push(close);
}

fn write_json(path: &Path, v: &Value) -> Result<(), String> {
    let mut s = String::new();
    layout(v, 0, &mut s);
    s.push('\n');
    fs::create_dir_all(path.parent().expect("a file in a directory")).map_err(|e| e.to_string())?;
    fs::write(path, s).map_err(|e| format!("{}: {e}", path.display()))
}

fn read_json(path: &Path) -> Result<Value, String> {
    let s = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&s).map_err(|e| format!("{}: {e}", path.display()))
}

/// "engine:pawn" lives at engine/pawn.json; a section with no owner under `_`.
fn section_file(name: &str) -> PathBuf {
    let (owner, rest) = name.split_once(':').unwrap_or(("", name));
    Path::new(if owner.is_empty() { "_" } else { owner }).join(format!("{rest}.json"))
}

fn sorted_dirs(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let rd = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut out: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    out.sort();
    Ok(out)
}

/// What `unpack` wrote.
#[derive(Debug)]
pub struct Unpacked {
    pub epochs: usize,
    pub snapshots: usize,
    /// Bytes at the end of the save that didn't make a whole chunk.
    pub cut: u64,
}

#[derive(Serialize, Deserialize)]
struct SnapshotFile {
    header: Header,
    /// `Snapshot::hash` as unpacked: pack tells an edited snapshot by it.
    hash: String,
}

/// Write a save as a directory of text. The directory must be new or empty.
pub fn unpack(save: &Path, dir: &Path) -> Result<Unpacked, String> {
    if fs::read_dir(dir).is_ok_and(|mut d| d.next().is_some()) {
        return Err(format!("{} isn't empty", dir.display()));
    }
    let (epochs, cut) = savefile::read(save)?;
    let mut out = Unpacked { epochs: epochs.len(), snapshots: 0, cut };
    for (n, e) in epochs.iter().enumerate() {
        let edir = dir.join(format!("epoch-{n}"));
        write_json(&edir.join("epoch.json"), &serde_json::to_value(&e.epoch).map_err(|e| e.to_string())?)?;
        let mut logs = serde_json::to_value(&e.logs).map_err(|e| e.to_string())?;
        name_defs("log", &mut logs, &defs_of(e.snapshots.first())?, true)?;
        write_json(&edir.join("log.json"), &logs)?;
        let mut used = std::collections::BTreeSet::new();
        for snap in &e.snapshots {
            // Two snapshots can share a tick: nothing ran between them.
            let base = format!("tick-{}", snap.header.tick);
            let mut names = (0..).map(|k| if k == 0 { base.clone() } else { format!("{base}-{k}") });
            let sdir = edir.join(names.find(|n| used.insert(n.clone())).expect("a free name"));
            let file = SnapshotFile { header: snap.header.clone(), hash: format!("{:016x}", snap.hash()) };
            write_json(&sdir.join("snapshot.json"), &serde_json::to_value(file).map_err(|e| e.to_string())?)?;
            for (name, v) in snapshot_text(snap)? {
                write_json(&sdir.join(section_file(&name)), &v)?;
            }
            out.snapshots += 1;
        }
    }
    Ok(out)
}

/// A snapshot directory back, and whether it was edited.
fn read_snapshot(dir: &Path) -> Result<(Snapshot, bool), String> {
    let file: SnapshotFile = serde_json::from_value(read_json(&dir.join("snapshot.json"))?)
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut texts = BTreeMap::new();
    for owner in sorted_dirs(dir)? {
        let rd = fs::read_dir(&owner).map_err(|e| format!("{}: {e}", owner.display()))?;
        for f in rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")) {
            let (o, s) = (owner.file_name(), f.file_stem());
            let (Some(o), Some(s)) = (o.and_then(|o| o.to_str()), s.and_then(|s| s.to_str())) else { continue };
            texts.insert(format!("{}:{s}", if o == "_" { "" } else { o }), read_json(&f)?);
        }
    }
    let defs: DefsSection = match texts.get("engine:defs") {
        Some(v) => serde_json::from_value(v.clone()).map_err(|e| format!("engine:defs: {e}"))?,
        None => DefsSection::new(),
    };
    let mut sections = BTreeMap::new();
    for (name, mut v) in texts {
        name_defs(&name, &mut v, &defs, false)?;
        if name == "engine:map" {
            v = serde_json::to_value(read_map(v, &defs)?).map_err(|e| e.to_string())?;
        }
        let bytes = (codec_of(&name)?.from)(v).map_err(|e| format!("{}: {name}: {e}", dir.display()))?;
        sections.insert(name, bytes);
    }
    let snap = Snapshot { header: file.header, sections };
    let edited = format!("{:016x}", snap.hash()) != file.hash;
    Ok((snap, edited))
}

/// Turn a directory from `unpack` back into a save. If a snapshot was
/// edited, it's the root of a new epoch at the end, and this is its tick.
pub fn pack(dir: &Path, save: &Path) -> Result<Option<u64>, String> {
    let mut epochs: Vec<EpochRead> = Vec::new();
    let mut edited = Vec::new();
    let mut edirs: Vec<(usize, PathBuf)> = sorted_dirs(dir)?
        .into_iter()
        .filter_map(|p| Some((p.file_name()?.to_str()?.strip_prefix("epoch-")?.parse().ok()?, p)))
        .collect();
    edirs.sort();
    for (_, edir) in edirs {
        let epoch: Epoch = serde_json::from_value(read_json(&edir.join("epoch.json"))?)
            .map_err(|e| format!("{}: epoch.json: {e}", edir.display()))?;
        let mut snaps = sorted_dirs(&edir)?.iter().map(|d| read_snapshot(d)).collect::<Result<Vec<_>, _>>()?;
        // The file's order is by tick; "tick-5" still sorts before "tick-5-1".
        snaps.sort_by_key(|(s, _)| s.header.tick);
        edited.extend(snaps.iter().enumerate().filter(|(_, (_, changed))| *changed).map(|(i, _)| (epochs.len(), i)));
        let snapshots: Vec<Snapshot> = snaps.into_iter().map(|(s, _)| s).collect();
        let mut logs = read_json(&edir.join("log.json"))?;
        name_defs("log", &mut logs, &defs_of(snapshots.first())?, false)?;
        let logs: Vec<Log> = serde_json::from_value(logs).map_err(|e| format!("{}: log.json: {e}", edir.display()))?;
        epochs.push(EpochRead { epoch, snapshots, logs });
    }
    if epochs.is_empty() {
        return Err(format!("{} has no epoch-N directories", dir.display()));
    }
    let root = match edited.as_slice() {
        [] => None,
        [(e, s)] => {
            let snap = epochs[*e].snapshots.remove(*s);
            let tick = snap.header.tick;
            let epoch = Epoch {
                mods: snap.header.mods.clone(),
                engine: env!("CARGO_PKG_VERSION").into(),
                root: Root::Snapshot,
            };
            epochs.push(EpochRead { epoch, snapshots: vec![snap], logs: Vec::new() });
            Some(tick)
        }
        _ => return Err(format!("{} snapshots were edited; edit one at a time", edited.len())),
    };
    savefile::write(save, &epochs).map_err(|e| format!("{}: {e}", save.display()))?;
    Ok(root)
}

/// A value for a message: short, on one line.
fn brief(v: Option<&Value>) -> String {
    let s = v.map_or("nothing".to_string(), |v| v.to_string());
    match s.char_indices().nth(60) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s,
    }
}

/// Where two values first differ, and how.
fn first_diff(a: Option<&Value>, b: Option<&Value>, at: String) -> Option<String> {
    if a == b {
        return None;
    }
    match (a, b) {
        (Some(Value::Object(x)), Some(Value::Object(y))) => {
            let keys: std::collections::BTreeSet<&String> = x.keys().chain(y.keys()).collect();
            keys.into_iter().find_map(|k| first_diff(x.get(k), y.get(k), format!("{at}.{k}")))
        }
        (Some(Value::Array(x)), Some(Value::Array(y))) => {
            (0..x.len().max(y.len())).find_map(|i| first_diff(x.get(i), y.get(i), format!("{at}[{i}]")))
        }
        (Some(Value::String(x)), Some(Value::String(y))) if x.len() > 60 && y.len() > 60 => {
            let col = x.chars().zip(y.chars()).take_while(|(p, q)| p == q).count();
            let at_col = |s: &str| s.chars().nth(col).map_or("the end".into(), |c| format!("{c:?}"));
            Some(format!("{at}, column {col}: {} ≠ {}", at_col(x), at_col(y)))
        }
        _ => Some(format!("{at}: {} ≠ {}", brief(a), brief(b))),
    }
}

/// A section of `[entity, value]` rows, by entity.
fn rows(v: &Value) -> Option<BTreeMap<u64, &Value>> {
    v.as_array()?.iter().map(|r| Some((r.get(0)?.as_u64()?, r.get(1)?))).collect()
}

/// The newest snapshot of each save, section by section: the first
/// difference in each section that differs, naming the entity where the
/// section holds entities. Empty when they hold the same state.
pub fn diff(a: &Path, b: &Path) -> Result<Vec<String>, String> {
    let newest = |p: &Path| -> Result<Snapshot, String> {
        let (epochs, _) = savefile::read(p)?;
        let snap = epochs.into_iter().rev().find_map(|mut e| e.snapshots.pop());
        snap.ok_or_else(|| format!("{}: no snapshot", p.display()))
    };
    let (sa, sb) = (newest(a)?, newest(b)?);
    let mut out = Vec::new();
    let (ha, hb) = (serde_json::to_value(&sa.header).ok(), serde_json::to_value(&sb.header).ok());
    out.extend(first_diff(ha.as_ref(), hb.as_ref(), "header".into()));
    let (ta, tb) = (snapshot_text(&sa)?, snapshot_text(&sb)?);
    let names: std::collections::BTreeSet<&String> = ta.keys().chain(tb.keys()).collect();
    for name in names {
        let (x, y) = (ta.get(name), tb.get(name));
        if x == y {
            continue;
        }
        let (Some(x), Some(y)) = (x, y) else {
            out.push(format!("{name}: only in {}", if x.is_some() { "the first" } else { "the second" }));
            continue;
        };
        let found = match (rows(x), rows(y)) {
            (Some(rx), Some(ry)) => {
                let ids: std::collections::BTreeSet<&u64> = rx.keys().chain(ry.keys()).collect();
                ids.into_iter().find_map(|id| {
                    let (p, q) = (rx.get(id).copied(), ry.get(id).copied());
                    match (p, q) {
                        (Some(_), Some(_)) => first_diff(p, q, format!("entity {id}")),
                        _ => Some(format!(
                            "entity {id}: only in {}",
                            if p.is_some() { "the first" } else { "the second" }
                        )),
                    }
                })
            }
            _ => first_diff(Some(x), Some(y), String::new()),
        };
        out.push(format!("{name}: {}", found.unwrap_or_default().trim_start_matches('.')));
    }
    Ok(out)
}
