//! Mod discovery, load ordering, def merging and patching.
//!
//! Pipeline:
//! 1. Read every `mods/*/mod.toml`, check API compatibility.
//! 2. Topologically sort on `depends` + `load_after`; ties break by id so the
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
    #[serde(default)]
    pub load_after: Vec<String>,
    #[serde(skip)]
    pub dir: PathBuf,
}

pub struct ScriptSource {
    pub mod_id: String,
    pub name: String,
    pub source: String,
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
    let mut warnings = Vec::new();
    let manifests = discover(mods_dir)?;
    let order = sort(manifests)?;

    let mut entries: Vec<Entry> = Vec::new();
    let mut index: HashMap<(String, String), usize> = HashMap::new();
    let mut set_by: HashMap<String, String> = HashMap::new();
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
            if p.get("remove").and_then(|v| v.as_bool()) == Some(true) {
                entries[i].removed = true;
            }
            if let Some(toml::Value::Table(set)) = p.get("set") {
                merge(&mut entries[i].value, set, target, &m.id, &mut set_by, &mut warnings);
            }
        }

        for file in files_with_ext(&m.dir.join("scripts"), "luau") {
            let name = file.file_name().unwrap().to_string_lossy().to_string();
            let source = fs::read_to_string(&file).map_err(|e| format!("{}/scripts/{name}: {e}", m.id))?;
            scripts.push(ScriptSource { mod_id: m.id.clone(), name, source });
        }
    }

    let mut defs = DefDb::default();
    for e in entries.into_iter().filter(|e| !e.removed) {
        let ctx = format!("{}/{} (from {})", e.kind, e.id, e.origin);
        let v = toml::Value::Table(e.value);
        let err = |x: toml::de::Error| format!("{ctx}: {}", x.message());
        match e.kind.as_str() {
            "terrain" => defs.terrain.push(v.try_into().map_err(err)?),
            "thing" => defs.things.push(v.try_into().map_err(err)?),
            "creature" => defs.creatures.push(v.try_into().map_err(err)?),
            "need" => defs.needs.push(v.try_into().map_err(err)?),
            "designation" => defs.designations.push(v.try_into().map_err(err)?),
            "field" => defs.fields.push(v.try_into().map_err(err)?),
            "start" => defs.start = Some(v.try_into().map_err(err)?),
            "names" => {
                let n: NamesDef = v.try_into().map_err(err)?;
                defs.names.extend(n.names);
            }
            _ => unreachable!(),
        }
    }
    defs.finalize()?;
    Ok(LoadedMods { mods: order, defs, scripts, warnings })
}

/// Deep-merge `src` into `dst`, recording which mod set each leaf field.
fn merge(
    dst: &mut toml::Table,
    src: &toml::Table,
    path: &str,
    mod_id: &str,
    set_by: &mut HashMap<String, String>,
    warnings: &mut Vec<String>,
) {
    for (k, v) in src {
        let p = format!("{path}.{k}");
        match (dst.get_mut(k), v) {
            (Some(toml::Value::Table(d)), toml::Value::Table(s)) => merge(d, s, &p, mod_id, set_by, warnings),
            _ => {
                if let Some(prev) = set_by.insert(p.clone(), mod_id.to_string()) {
                    if prev != mod_id {
                        warnings.push(format!(
                            "patch conflict: {p} set by both '{prev}' and '{mod_id}' ('{mod_id}' wins by load order)"
                        ));
                    }
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
        for d in &m.load_after {
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

fn files_with_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == ext)).collect())
        .unwrap_or_default();
    v.sort();
    v
}
