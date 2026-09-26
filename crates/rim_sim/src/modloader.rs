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

use crate::data::{Data, Key};
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
    /// The UI scripting surface the mod's `ui/` scripts were written
    /// against, when it has any. Checked by the UI engine and `rim check`.
    #[serde(default)]
    pub ui_api: Option<String>,
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

    let mut kinds: BTreeMap<String, KindDecl> = BTreeMap::new();
    for m in &order {
        let mut patches: Vec<(String, toml::Table)> = Vec::new();
        // Parse every file first, so a kind declared in one serves them all.
        let mut files: Vec<(String, toml::Table)> = Vec::new();
        for file in files_with_ext(&m.dir.join("defs"), "toml") {
            let fname = file.file_name().unwrap().to_string_lossy().to_string();
            let origin = format!("{}/defs/{}", m.id, fname);
            let text = fs::read_to_string(&file).map_err(|e| format!("{origin}: {e}"))?;
            let table: toml::Table = text.parse().map_err(|e| format!("{origin}: {e}"))?;
            files.push((origin, table));
        }
        for (origin, table) in &mut files {
            if let Some(v) = table.remove("kind") {
                declare_kinds(v, origin, &m.id, &mut kinds)?;
            }
        }
        for (origin, table) in files {
            // (kind, entries): `[[thing]]`, `[[type]]` for this mod's own kind,
            // or `[[weather.type]]`, which TOML reads as weather = { type = [...] }.
            let mut groups: Vec<(String, toml::Value)> = Vec::new();
            for (key, val) in table {
                match val {
                    toml::Value::Table(sub) if !KINDS.contains(&key.as_str()) && key != "patch" => {
                        for (name, v) in sub {
                            let full = format!("{key}:{name}");
                            if kinds.contains_key(&full) {
                                groups.push((full, v));
                            } else {
                                warnings.push(format!("{origin}: unknown def kind '{key}.{name}' ignored"));
                            }
                        }
                    }
                    v => groups.push((key, v)),
                }
            }
            for (key, val) in groups {
                let arr = match val {
                    toml::Value::Array(a) => a,
                    _ => return Err(format!("{origin}: '{key}' must be an array of tables ([[{key}]])")),
                };
                let own = format!("{}:{key}", m.id);
                let kind = if key == "patch" || KINDS.contains(&key.as_str()) || key.contains(':') {
                    key.clone()
                } else if kinds.contains_key(&own) {
                    own
                } else {
                    let elsewhere: Vec<String> = kinds
                        .keys()
                        .filter(|k| k.split_once(':').is_some_and(|(_, b)| b == key))
                        .map(|k| format!("[[{}]]", k.replace(':', ".")))
                        .collect();
                    let hint = if elsewhere.is_empty() {
                        String::new()
                    } else {
                        format!(" (did you mean {}?)", elsewhere.join(" or "))
                    };
                    warnings.push(format!("{origin}: unknown def kind '{key}' ignored{hint}"));
                    continue;
                };
                for v in arr {
                    let toml::Value::Table(t) = v else {
                        return Err(format!("{origin}: '{kind}' entries must be tables"));
                    };
                    if kind == "patch" {
                        patches.push((origin.clone(), t));
                        continue;
                    }
                    let raw = t
                        .get("id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| format!("{origin}: a [[{key}]] entry has no id"))?;
                    // Every id is the mod's own: "wall" in core is "core:wall".
                    let id = match raw.split_once(':') {
                        None => format!("{}:{raw}", m.id),
                        Some((owner, _)) if owner == m.id => raw.to_string(),
                        Some((owner, bare)) => {
                            return Err(format!(
                                "{origin}: [[{key}]] id '{raw}' is in mod '{owner}'s namespace; a mod defines \
                                 its own ids (id = \"{bare}\"), and changes another mod's with a [[patch]]"
                            ))
                        }
                    };
                    let mut t = t;
                    t.insert("id".into(), toml::Value::String(id.clone()));
                    let entry_key = (kind.clone(), id.clone());
                    if let Some(&i) = index.get(&entry_key) {
                        return Err(format!(
                            "{origin}: {kind}/{id} is already defined by {} — use a [[patch]] to change it",
                            entries[i].origin
                        ));
                    }
                    index.insert(entry_key, entries.len());
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
            // A mod kind is named like an id: `type` in its own mod, else `weather:type`.
            let own_kind = format!("{}:{kind}", m.id);
            let kind = if !KINDS.contains(&kind) && !kind.contains(':') && kinds.contains_key(&own_kind) {
                own_kind.as_str()
            } else {
                kind
            };
            // A bare id is the patching mod's own, like any reference.
            let full = if id.contains(':') { id.to_string() } else { format!("{}:{id}", m.id) };
            let Some(&i) = index.get(&(kind.to_string(), full.clone())) else {
                let mut others: Vec<&String> = index
                    .keys()
                    .filter(|(k, other)| {
                        k == kind && !id.contains(':') && other.split_once(':').is_some_and(|(_, b)| b == id)
                    })
                    .map(|(_, other)| other)
                    .collect();
                others.sort();
                if let Some(other) = others.first() {
                    return Err(format!(
                        "{origin}: patch target {target}: that's another mod's def, so name it with its prefix: \
                         target = \"{kind}/{other}\""
                    ));
                }
                // Patching an optional mod that isn't installed is normal.
                warnings.push(format!("{origin}: patch target {kind}/{full} not found (skipped)"));
                continue;
            };
            let target = format!("{kind}/{full}");
            apply_patch(&mut entries[i], &p, &origin, &target, &m.id, &mut log)?;
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
    let mut scales = 0;
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
            "work_type" => defs.work_types.push(de!(v)?),
            "work_style" => defs.work_styles.push(de!(v)?),
            "skill" => defs.skills.push(de!(v)?),
            "priority_scale" => {
                if scales > 0 {
                    return Err(format!(
                        "{ctx}: only one [[priority_scale]] may exist; patch priority_scale/{} instead",
                        defs.priority_scale.id
                    ));
                }
                scales += 1;
                defs.priority_scale = de!(v)?;
            }
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
            mod_kind => {
                let decl = &kinds[mod_kind];
                let toml::Value::Table(t) = v else { unreachable!() };
                let d = check_fields(decl, t, &ctx, &mut warnings)?;
                defs.mod_defs.entry(mod_kind.to_string()).or_default().push(d);
            }
        }
    }
    for k in kinds.keys() {
        defs.mod_defs.entry(k.clone()).or_default();
    }
    defs.finalize()?;
    defs.sprite_files = sprite_files(&order, &defs)?;
    warnings.extend(log.warnings);
    Ok(LoadedMods { mods: order, defs, scripts, warnings })
}

/// The PNG behind each sprite key: `<mod>/sprites/<name>.png`. A key with
/// no file is a load error naming the def that asked for it.
fn sprite_files(mods: &[ModManifest], defs: &DefDb) -> Result<Vec<PathBuf>, String> {
    defs.sprites
        .iter()
        .enumerate()
        .map(|(id, key)| {
            let (m, name) = key.split_once(':').unwrap_or(("", key));
            // A name is a path inside the mod's sprites/, and only inside it.
            let inside = !name.is_empty()
                && !name.contains('\\')
                && name.split('/').all(|part| !part.is_empty() && part != "." && part != ".." && !part.contains(':'));
            if !inside {
                return Err(format!(
                    "sprite '{key}': a name is a path inside the mod's sprites/, without '..', '\\' or a root"
                ));
            }
            let file = mods.iter().find(|x| x.id == m).map(|x| x.dir.join("sprites").join(format!("{name}.png")));
            match file {
                Some(f) if f.is_file() => Ok(f),
                _ => {
                    let user =
                        defs.things
                            .iter()
                            .find(|t| {
                                t.look_r.layers.iter().chain(&t.look_r.regrowing).any(
                                    |l| matches!(l.prim, crate::look::Prim::Sprite { id: i, .. } if i as usize == id),
                                )
                            })
                            .map_or(String::new(), |t| format!("thing/{}: ", t.id));
                    let wanted = if mods.iter().any(|x| x.id == m) {
                        format!("{m}/sprites/{name}.png")
                    } else {
                        format!("no mod '{m}' is loaded")
                    };
                    Err(format!("{user}unknown sprite '{key}' ({wanted})"))
                }
            }
        })
        .collect()
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

/// A def kind a mod declared with `[[kind]]`.
struct KindDecl {
    /// Field name to its type ("string", "int", "float", "bool", "table",
    /// "list", "any") and default. None: any fields at all.
    fields: Option<BTreeMap<String, (String, Option<toml::Value>)>>,
}

const FIELD_TYPES: &[&str] = &["string", "int", "float", "bool", "table", "list", "any"];

fn declare_kinds(
    v: toml::Value,
    origin: &str,
    mod_id: &str,
    kinds: &mut BTreeMap<String, KindDecl>,
) -> Result<(), String> {
    let toml::Value::Array(arr) = v else {
        return Err(format!("{origin}: 'kind' must be an array of tables ([[kind]])"));
    };
    for k in arr {
        let toml::Value::Table(k) = k else { return Err(format!("{origin}: [[kind]] entries must be tables")) };
        if let Some(bad) = k.keys().find(|x| !["id", "fields"].contains(&x.as_str())) {
            return Err(format!("{origin}: [[kind]] has an unknown key '{bad}' (id, fields)"));
        }
        let raw = k.get("id").and_then(|v| v.as_str()).ok_or_else(|| format!("{origin}: a [[kind]] has no id"))?;
        let bare = match raw.split_once(':') {
            None => raw,
            Some((owner, b)) if owner == mod_id => b,
            Some(_) => return Err(format!("{origin}: [[kind]] '{raw}': a mod declares kinds in its own namespace")),
        };
        if KINDS.contains(&bare) || bare == "patch" || bare == "kind" {
            return Err(format!("{origin}: [[kind]] '{bare}' is a built-in kind's name; pick another"));
        }
        let id = format!("{mod_id}:{bare}");
        let fields = match k.get("fields") {
            None => None,
            Some(toml::Value::Table(f)) => {
                let mut out = BTreeMap::new();
                for (name, spec) in f {
                    let (ty, default) = match spec {
                        toml::Value::String(t) => (t.clone(), None),
                        toml::Value::Table(t) => (
                            t.get("type").and_then(|v| v.as_str()).unwrap_or("any").to_string(),
                            t.get("default").cloned(),
                        ),
                        _ => {
                            return Err(format!(
                                "{origin}: kind {id}, field '{name}': give a type or {{ type, default }}"
                            ))
                        }
                    };
                    if !FIELD_TYPES.contains(&ty.as_str()) {
                        return Err(format!(
                            "{origin}: kind {id}, field '{name}': unknown type '{ty}' (one of {})",
                            FIELD_TYPES.join(", ")
                        ));
                    }
                    out.insert(name.clone(), (ty, default));
                }
                Some(out)
            }
            Some(_) => return Err(format!("{origin}: kind {id}: 'fields' must be a table")),
        };
        if kinds.insert(id.clone(), KindDecl { fields }).is_some() {
            return Err(format!("{origin}: kind {id} is declared twice"));
        }
    }
    Ok(())
}

fn type_ok(ty: &str, v: &toml::Value) -> bool {
    matches!(
        (ty, v),
        ("any", _)
            | ("string", toml::Value::String(_))
            | ("int", toml::Value::Integer(_))
            | ("float", toml::Value::Float(_) | toml::Value::Integer(_))
            | ("bool", toml::Value::Boolean(_))
            | ("table", toml::Value::Table(_))
            | ("list", toml::Value::Array(_))
    )
}

/// A mod kind's entry, checked against its declared fields, as plain data.
fn check_fields(decl: &KindDecl, mut t: toml::Table, ctx: &str, warnings: &mut Vec<String>) -> Result<Data, String> {
    if let Some(fields) = &decl.fields {
        for (name, (ty, default)) in fields {
            match (t.get(name), default) {
                (Some(v), _) if !type_ok(ty, v) => {
                    return Err(format!("{ctx}: `{name}` should be {ty}, not {}", v.type_str()));
                }
                (Some(_), _) => {}
                (None, Some(d)) => {
                    t.insert(name.clone(), d.clone());
                }
                (None, None) => return Err(format!("{ctx}: missing field `{name}` ({ty})")),
            }
        }
        for k in t.keys().filter(|k| *k != "id" && !fields.contains_key(*k)) {
            warnings.push(format!("{ctx}: `{k}` isn't a field of this kind (ignored by its schema)"));
        }
    }
    Ok(toml_data(&toml::Value::Table(t)))
}

/// TOML as script data: arrays become 1-based lists.
fn toml_data(v: &toml::Value) -> Data {
    match v {
        toml::Value::String(s) => Data::Str(s.clone()),
        toml::Value::Integer(i) => Data::Int(*i),
        toml::Value::Float(f) => Data::Num(*f),
        toml::Value::Boolean(b) => Data::Bool(*b),
        toml::Value::Datetime(d) => Data::Str(d.to_string()),
        toml::Value::Array(a) => {
            Data::Table(a.iter().enumerate().map(|(i, x)| (Key::Int(i as i64 + 1), toml_data(x))).collect())
        }
        toml::Value::Table(t) => Data::Table(t.iter().map(|(k, x)| (Key::Str(k.clone()), toml_data(x))).collect()),
    }
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

/// A field that takes one table or a list of them (`harvest`) is patched
/// as a list: a single table becomes a list of one.
fn listify(v: &mut toml::Value) {
    if let toml::Value::Table(t) = v {
        *v = toml::Value::Array(vec![toml::Value::Table(std::mem::take(t))]);
    }
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
                listify(entry);
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
    let not_list = || format!("{origin}: edit: {target}.{list_path} isn't a list");
    let list = loop {
        let k = keys.next().unwrap_or_default();
        if keys.peek().is_some() {
            let Some(toml::Value::Table(t)) = node.get_mut(k) else { return Err(not_list()) };
            node = t;
            continue;
        }
        let Some(v) = node.get_mut(k) else { return Err(not_list()) };
        listify(v);
        let toml::Value::Array(a) = v else { return Err(not_list()) };
        break a;
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
            // A one-or-many field another patch made a list: a table still
            // merges into a list of one. With several there's no telling
            // which one it means.
            (Some(toml::Value::Array(list)), toml::Value::Table(s)) if list.iter().all(|e| e.is_table()) => {
                match list.as_mut_slice() {
                    [toml::Value::Table(only)] => merge(only, s, &p, mod_id, log),
                    _ => log.warnings.push(format!(
                        "patch: '{mod_id}' sets {p}, which holds {} entries, so a table can't say which; \
                         use [[patch.edit]] (skipped)",
                        list.len()
                    )),
                }
            }
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

pub(crate) fn discover(dir: &Path) -> Result<Vec<ModManifest>, String> {
    let rd = fs::read_dir(dir).map_err(|e| format!("cannot read mods dir {}: {e}", dir.display()))?;
    let mut out = Vec::new();
    for ent in rd.flatten() {
        if !ent.path().join("mod.toml").is_file() {
            continue;
        }
        out.push(read_manifest(&ent.path())?);
    }
    Ok(out)
}

/// One mod's `mod.toml`, checked against the sim API version.
pub fn read_manifest(mod_dir: &Path) -> Result<ModManifest, String> {
    let mf = mod_dir.join("mod.toml");
    let text = fs::read_to_string(&mf).map_err(|e| format!("{}: {e}", mf.display()))?;
    let mut m: ModManifest = toml::from_str(&text).map_err(|e| format!("{}: {}", mf.display(), e.message()))?;
    check_api(&m)?;
    m.dir = mod_dir.to_path_buf();
    Ok(m)
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
