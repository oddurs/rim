//! Mod discovery, load ordering, def merging and patching.
//!
//! Pipeline:
//! 1. Read every `mods/*/mod.toml`, check API compatibility.
//! 2. Topologically sort on `depends`, `optional` and `load_after`; ties break by id so the
//!    order is deterministic across machines.
//! 3. For each mod in order: add its defs (redefining another mod's def is an
//!    error — use a patch), then apply its `[[patch]]` entries.
//! 4. Deserialize into typed defs and resolve references.
//!
//! Patch conflicts (two mods setting the same field) are reported, not
//! silently resolved by whoever loaded last.

use crate::defs::*;
use crate::API_VERSION;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Clone, Debug)]
pub struct ModManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub depends: Vec<String>,
    /// Mods this one uses when they're installed: it may `require` them, and
    /// loads after them if they're there.
    #[serde(default)]
    pub optional: Vec<String>,
    #[serde(default)]
    pub load_after: Vec<String>,
    #[serde(skip)]
    pub dir: PathBuf,
}

pub struct ScriptSource {
    pub mod_id: String,
    /// Path under the mod's `scripts/`, with `/` separators: `storyteller.luau`, `lib/util.luau`.
    pub name: String,
    pub source: String,
    /// Top-level scripts run at load, in name order. Scripts in
    /// subdirectories are modules: they run only when something requires them.
    pub entry: bool,
}

pub struct LoadedMods {
    pub mods: Vec<ModManifest>,
    pub defs: DefDb,
    pub scripts: Vec<ScriptSource>,
    pub warnings: Vec<String>,
}

struct Entry {
    kind: String,
    id: String,
    value: toml::Table,
    origin: String,
    removed: bool,
}

pub fn load(mods_dir: &Path) -> Result<LoadedMods, String> {
    load_only(mods_dir, &|_| true)
}

/// Load only the mods `enabled` accepts (by id). Tests use it to run core
/// alone; a mod list in the launcher will too.
pub fn load_only(mods_dir: &Path, enabled: &dyn Fn(&str) -> bool) -> Result<LoadedMods, String> {
    let mut warnings = Vec::new();
    let manifests: Vec<ModManifest> = discover(mods_dir)?.into_iter().filter(|m| enabled(&m.id)).collect();
    let order = sort(manifests)?;

    let mut entries: Vec<Entry> = Vec::new();
    let mut index: HashMap<(String, String), usize> = HashMap::new();
    let mut log = PatchLog::default();
    let mut scripts = Vec::new();

    for m in &order {
        let mut patches: Vec<(String, toml::Table)> = Vec::new();
        for file in files_with_ext(&m.dir.join("defs"), "toml") {
            let fname = file.file_name().unwrap().to_string_lossy().to_string();
            let origin = format!("{}/defs/{}", m.id, fname);
            let text = fs::read_to_string(&file).map_err(|e| format!("{origin}: {e}"))?;
            let table: toml::Table = text.parse().map_err(|e| format!("{origin}: {e}"))?;
            for (kind, val) in table {
                let arr = match val {
                    toml::Value::Array(a) => a,
                    _ => return Err(format!("{origin}: '{kind}' must be an array of tables ([[{kind}]])")),
                };
                for v in arr {
                    let toml::Value::Table(t) = v else {
                        return Err(format!("{origin}: '{kind}' entries must be tables"));
                    };
                    if kind == "patch" {
                        patches.push((origin.clone(), t));
                        continue;
                    }
                    if !KINDS.contains(&kind.as_str()) {
                        warnings.push(format!("{origin}: unknown def kind '{kind}' ignored"));
                        continue;
                    }
                    let id = t
                        .get("id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| format!("{origin}: a [[{kind}]] entry has no id"))?
                        .to_string();
                    let key = (kind.clone(), id.clone());
                    if let Some(&i) = index.get(&key) {
                        return Err(format!(
                            "{origin}: {kind}/{id} is already defined by {} — use a [[patch]] to change it",
                            entries[i].origin
                        ));
                    }
                    index.insert(key, entries.len());
                    entries.push(Entry { kind: kind.clone(), id, value: t, origin: origin.clone(), removed: false });
                }
            }
        }

        for (origin, p) in patches {
            let target = p
                .get("target")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("{origin}: patch has no 'target' (\"kind/id\")"))?;
            let Some((kind, id)) = target.split_once('/') else {
                return Err(format!("{origin}: patch target '{target}' must look like kind/id"));
            };
            let Some(&i) = index.get(&(kind.to_string(), id.to_string())) else {
                // Patching an optional mod that isn't installed is normal.
                warnings.push(format!("{origin}: patch target {target} not found (skipped)"));
                continue;
            };
            apply_patch(&mut entries[i], &p, &origin, target, &m.id, &mut log)?;
        }

        let root = m.dir.join("scripts");
        for file in luau_files(&root) {
            let rel = file.strip_prefix(&root).unwrap();
            let entry = rel.components().count() == 1;
            let name = rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
            let source = fs::read_to_string(&file).map_err(|e| format!("{}/scripts/{name}: {e}", m.id))?;
            scripts.push(ScriptSource { mod_id: m.id.clone(), name, source, entry });
        }
    }

    let mut defs = DefDb::default();
    let mut calendars = 0;
    let mut skies = 0;
    for e in entries.into_iter().filter(|e| !e.removed) {
        let ctx = format!("{}/{} (from {})", e.kind, e.id, e.origin);
        let v = toml::Value::Table(e.value);
        let target = format!("{}/{}", e.kind, e.id);
        // A macro, not a closure: each arm deserializes to a different type.
        macro_rules! de {
            ($v:expr) => {
                deserialize($v, &ctx, &target, &log)
            };
        }
        match e.kind.as_str() {
            "terrain" => defs.terrain.push(de!(v)?),
            "thing" => defs.things.push(de!(v)?),
            "creature" => defs.creatures.push(de!(v)?),
            "need" => defs.needs.push(de!(v)?),
            "designation" => defs.designations.push(de!(v)?),
            "field" => defs.fields.push(de!(v)?),
            "calendar" => {
                if calendars > 0 {
                    return Err(format!(
                        "{ctx}: only one [[calendar]] may exist; patch calendar/{} instead",
                        defs.calendar.id
                    ));
                }
                calendars += 1;
                defs.calendar = de!(v)?;
            }
            "sky" => {
                if skies > 0 {
                    return Err(format!("{ctx}: only one [[sky]] may exist; patch sky/{} instead", defs.sky.id));
                }
                skies += 1;
                defs.sky = de!(v)?;
            }
            "start" => defs.start = Some(de!(v)?),
            "names" => {
                let n: NamesDef = de!(v)?;
                defs.names.extend(n.names);
            }
            _ => unreachable!(),
        }
    }
    defs.finalize()?;
    warnings.extend(log.warnings);
    Ok(LoadedMods { mods: order, defs, scripts, warnings })
}

/// Deserialize one def, and on failure say exactly where: the key path
/// inside the def (`build.cost[0].count`) and, if a patch set that key, which
/// mod's patch it was.
fn deserialize<T: serde::de::DeserializeOwned>(
    v: toml::Value,
    ctx: &str,
    target: &str,
    log: &PatchLog,
) -> Result<T, String> {
    serde_path_to_error::deserialize(v).map_err(|e| {
        let path = e.path().to_string();
        let msg = e.into_inner().message().to_string();
        if path.is_empty() || path == "." {
            return format!("{ctx}: {msg}");
        }
        // The patch record keys look like `thing/wall.build.cost`: try the
        // failing path and each of its parents, without list indices.
        let plain: Vec<&str> =
            path.split('.').map(|seg| seg.split('[').next().unwrap_or(seg)).filter(|s| !s.is_empty()).collect();
        let patched = (1..=plain.len())
            .rev()
            .find_map(|n| {
                let key = format!("{target}.{}", plain[..n].join("."));
                log.set_by.get(&key).or_else(|| log.list_by.get(&key))
            })
            .map(|m| format!(" (patched by '{m}')"))
            .unwrap_or_default();
        format!("{ctx}: at `{path}`{patched}: {msg}")
    })
}

/// What patches did, for conflict warnings and for naming the mod behind a
/// bad value. Keys are paths like `thing/wall.build.cost`.
#[derive(Default)]
struct PatchLog {
    /// The mod whose `set` wrote each field.
    set_by: HashMap<String, String>,
    /// The last mod to append to, remove from or edit each list.
    list_by: BTreeMap<String, String>,
    warnings: Vec<String>,
}

const PATCH_KEYS: &[&str] = &["target", "set", "remove", "append", "edit"];

/// One `[[patch]]` on its target def, in a fixed order: `set` replaces
/// fields, `edit` changes matched elements of a list, `remove` takes
/// elements out, `append` adds them.
fn apply_patch(
    e: &mut Entry,
    p: &toml::Table,
    origin: &str,
    target: &str,
    mod_id: &str,
    log: &mut PatchLog,
) -> Result<(), String> {
    if let Some(k) = p.keys().find(|k| !PATCH_KEYS.contains(&k.as_str())) {
        return Err(format!("{origin}: patch on {target} has an unknown key '{k}' (one of {})", PATCH_KEYS.join(", ")));
    }
    if let Some(v) = p.get("set") {
        let toml::Value::Table(set) = v else { return Err(format!("{origin}: 'set' must be a table")) };
        merge(&mut e.value, set, target, mod_id, log);
    }
    match p.get("edit") {
        None => {}
        Some(toml::Value::Array(edits)) => {
            for ed in edits {
                let toml::Value::Table(ed) = ed else { return Err(format!("{origin}: each 'edit' must be a table")) };
                edit_list(&mut e.value, ed, origin, target, mod_id, log)?;
            }
        }
        Some(_) => return Err(format!("{origin}: 'edit' must be a list of tables ([[patch.edit]])")),
    }
    match p.get("remove") {
        None => {}
        Some(toml::Value::Boolean(b)) => e.removed |= b,
        Some(toml::Value::Table(t)) => list_op(&mut e.value, t, target, origin, mod_id, false, log)?,
        Some(_) => return Err(format!("{origin}: 'remove' is true (remove the def) or a table of list elements")),
    }
    match p.get("append") {
        None => {}
        Some(toml::Value::Table(t)) => list_op(&mut e.value, t, target, origin, mod_id, true, log)?,
        Some(_) => return Err(format!("{origin}: 'append' must be a table of lists")),
    }
    Ok(())
}

/// Does a list element match a pattern? A table pattern matches a table
/// that has all its keys with equal values; anything else must be equal.
fn matches(el: &toml::Value, pattern: &toml::Value) -> bool {
    match (el, pattern) {
        (toml::Value::Table(e), toml::Value::Table(pat)) => pat.iter().all(|(k, v)| e.get(k) == Some(v)),
        _ => el == pattern,
    }
}

/// `append` or `remove`: `src` mirrors the def's shape down to lists.
fn list_op(
    dst: &mut toml::Table,
    src: &toml::Table,
    path: &str,
    origin: &str,
    mod_id: &str,
    append: bool,
    log: &mut PatchLog,
) -> Result<(), String> {
    let verb = if append { "append" } else { "remove" };
    for (k, v) in src {
        let p = format!("{path}.{k}");
        match v {
            toml::Value::Table(sub) => {
                let entry = dst.entry(k.clone()).or_insert_with(|| toml::Value::Table(toml::Table::new()));
                let toml::Value::Table(d) = entry else {
                    return Err(format!("{origin}: can't {verb} inside {p}: it isn't a table"));
                };
                list_op(d, sub, &p, origin, mod_id, append, log)?;
            }
            toml::Value::Array(items) => {
                let entry = dst.entry(k.clone()).or_insert_with(|| toml::Value::Array(Vec::new()));
                let toml::Value::Array(list) = entry else {
                    return Err(format!("{origin}: can't {verb} to {p}: it isn't a list"));
                };
                if append {
                    list.extend(items.iter().cloned());
                } else {
                    for pat in items {
                        let before = list.len();
                        list.retain(|el| !matches(el, pat));
                        if list.len() == before {
                            log.warnings.push(format!("{origin}: remove from {p}: nothing matched {pat}"));
                        }
                    }
                }
                log.list_by.insert(p, mod_id.to_string());
            }
            _ => return Err(format!("{origin}: {verb} {p}: give a list of elements")),
        }
    }
    Ok(())
}

/// `[[patch.edit]]`: `set` merged into each element of `list` that `match`es.
fn edit_list(
    dst: &mut toml::Table,
    ed: &toml::Table,
    origin: &str,
    target: &str,
    mod_id: &str,
    log: &mut PatchLog,
) -> Result<(), String> {
    if let Some(k) = ed.keys().find(|k| !["list", "match", "set"].contains(&k.as_str())) {
        return Err(format!("{origin}: edit on {target} has an unknown key '{k}' (list, match, set)"));
    }
    let list_path = ed
        .get("list")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("{origin}: edit needs list = \"path.to.list\""))?;
    let Some(toml::Value::Table(pat)) = ed.get("match") else {
        return Err(format!("{origin}: edit of {list_path} needs match = {{ key = value }}"));
    };
    let Some(toml::Value::Table(set)) = ed.get("set") else {
        return Err(format!("{origin}: edit of {list_path} needs set = {{ ... }}"));
    };
    let mut node = dst;
    let mut keys = list_path.split('.').peekable();
    let list = loop {
        let k = keys.next().unwrap_or_default();
        match (node.get_mut(k), keys.peek().is_some()) {
            (Some(toml::Value::Table(t)), true) => node = t,
            (Some(toml::Value::Array(a)), false) => break a,
            _ => return Err(format!("{origin}: edit: {target}.{list_path} isn't a list")),
        }
    };
    let key: Vec<String> = pat.iter().map(|(k, v)| format!("{k}={v}")).collect();
    let el_path = format!("{target}.{list_path}[{}]", key.join(","));
    let pattern = toml::Value::Table(pat.clone());
    let mut hit = false;
    for el in list.iter_mut().filter(|el| matches(el, &pattern)) {
        let toml::Value::Table(t) = el else { continue };
        merge(t, set, &el_path, mod_id, log);
        hit = true;
    }
    if !hit {
        log.warnings.push(format!("{origin}: edit of {el_path}: no element matched (skipped)"));
    }
    log.list_by.insert(format!("{target}.{list_path}"), mod_id.to_string());
    Ok(())
}

/// Deep-merge `src` into `dst`, recording which mod set each leaf field.
/// Replacing a list another mod edited is reported: their edits are lost.
fn merge(dst: &mut toml::Table, src: &toml::Table, path: &str, mod_id: &str, log: &mut PatchLog) {
    for (k, v) in src {
        let p = format!("{path}.{k}");
        match (dst.get_mut(k), v) {
            (Some(toml::Value::Table(d)), toml::Value::Table(s)) => merge(d, s, &p, mod_id, log),
            _ => {
                if let Some(prev) = log.set_by.insert(p.clone(), mod_id.to_string()) {
                    if prev != mod_id {
                        log.warnings.push(format!(
                            "patch conflict: {p} set by both '{prev}' and '{mod_id}' ('{mod_id}' wins by load order)"
                        ));
                    }
                }
                if let Some(prev) = log.list_by.get(&p).filter(|m| *m != mod_id) {
                    log.warnings
                        .push(format!("patch conflict: '{mod_id}' sets {p}, replacing the list '{prev}' edited"));
                }
                dst.insert(k.clone(), v.clone());
            }
        }
    }
}

fn discover(dir: &Path) -> Result<Vec<ModManifest>, String> {
    let rd = fs::read_dir(dir).map_err(|e| format!("cannot read mods dir {}: {e}", dir.display()))?;
    let mut out = Vec::new();
    for ent in rd.flatten() {
        let mf = ent.path().join("mod.toml");
        if !mf.is_file() {
            continue;
        }
        let text = fs::read_to_string(&mf).map_err(|e| format!("{}: {e}", mf.display()))?;
        let mut m: ModManifest = toml::from_str(&text).map_err(|e| format!("{}: {}", mf.display(), e.message()))?;
        check_api(&m)?;
        m.dir = ent.path();
        out.push(m);
    }
    Ok(out)
}

fn check_api(m: &ModManifest) -> Result<(), String> {
    let parse = |s: &str| -> Option<(u32, u32)> {
        let (a, b) = s.split_once('.')?;
        Some((a.parse().ok()?, b.parse().ok()?))
    };
    let (maj, min) = parse(&m.api).ok_or_else(|| format!("mod '{}': bad api version '{}'", m.id, m.api))?;
    // Semver: before 1.0 every minor is breaking; after, minors are additive.
    let ok = maj == API_VERSION.0 && if maj == 0 { min == API_VERSION.1 } else { min <= API_VERSION.1 };
    if !ok {
        return Err(format!(
            "mod '{}' targets api {} but the engine provides {}.{}",
            m.id, m.api, API_VERSION.0, API_VERSION.1
        ));
    }
    Ok(())
}

fn sort(mods: Vec<ModManifest>) -> Result<Vec<ModManifest>, String> {
    let mut by_id: BTreeMap<String, ModManifest> = BTreeMap::new();
    for m in mods {
        if by_id.contains_key(&m.id) {
            return Err(format!("two mods share the id '{}'", m.id));
        }
        by_id.insert(m.id.clone(), m);
    }
    let mut incoming: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for m in by_id.values() {
        let mut before = BTreeSet::new();
        for d in &m.depends {
            if !by_id.contains_key(d) {
                return Err(format!("mod '{}' requires '{}', which is not installed", m.id, d));
            }
            before.insert(d.clone());
        }
        for d in m.optional.iter().chain(&m.load_after) {
            if by_id.contains_key(d) {
                before.insert(d.clone());
            }
        }
        incoming.insert(m.id.clone(), before);
    }
    let mut order = Vec::new();
    while !incoming.is_empty() {
        let ready = incoming.iter().find(|(_, deps)| deps.is_empty()).map(|(k, _)| k.clone());
        let Some(id) = ready else {
            let stuck: Vec<_> = incoming.keys().cloned().collect();
            return Err(format!("mod dependency cycle among: {}", stuck.join(", ")));
        };
        incoming.remove(&id);
        for deps in incoming.values_mut() {
            deps.remove(&id);
        }
        order.push(by_id.remove(&id).unwrap());
    }
    Ok(order)
}

/// Every `.luau` file under `dir`, at any depth, in a fixed order.
fn luau_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = files_with_ext(dir, "luau");
    let mut subdirs: Vec<PathBuf> =
        fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
    subdirs.sort();
    for d in subdirs {
        out.extend(luau_files(&d));
    }
    out
}

fn files_with_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == ext)).collect())
        .unwrap_or_default();
    v.sort();
    v
}
