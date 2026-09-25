//! Snapshots: the whole world at a tick, as named sections (DESIGN.md §7a).
//!
//! A section belongs to the engine (`engine:pawn`, `engine:map`) or to a mod
//! (`weather:data`), and holds plain data in a canonical encoding: the same
//! world always gives the same bytes, so a section's hash is its identity.
//! Nothing derived is kept. Map layers, regions, rooms, field stamps and the
//! pawn list are rebuilt on load, from the things in the world.

use crate::data::Data;
use crate::defs::{Category, DefDb, DefId};
use crate::field::SavedFields;
use crate::sim::Sim;
use crate::world::*;
use hecs::{Entity, EntityBuilder};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Bumped whenever an `engine:` section changes shape.
pub const FORMAT: u32 = 1;

const MAGIC: &[u8; 8] = b"rimsnap1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Header {
    pub format: u32,
    pub engine: String,
    pub api: String,
    pub seed: u64,
    pub tick: u64,
    /// Every loaded mod's (id, version), in load order.
    pub mods: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub header: Header,
    /// Canonical bytes by section name.
    pub sections: BTreeMap<String, Vec<u8>>,
}

/// The world's own state: everything outside the map, fields and entities.
#[derive(Serialize, Deserialize)]
pub(crate) struct WorldSection {
    width: i32,
    height: i32,
    rng: u64,
    next_entity: u32,
    wealth: f64,
    colony_lost: bool,
    /// (target, holder), in target order.
    reservations: Vec<(Entity, Entity)>,
    messages: Vec<Message>,
    recent_events: Vec<(u64, String, Entity, String)>,
    /// Events raised after the last dispatch (a handler's `rim.emit`),
    /// delivered on the next tick.
    events: Vec<GameEvent>,
    /// `World::data_versions`: removed mods' data and its version.
    data_versions: BTreeMap<String, String>,
}

/// Each def kind's qualified ids, in `DefId` order: the table the raw ids in
/// every other section index into.
pub(crate) type DefsSection = BTreeMap<String, Vec<String>>;

#[derive(Serialize, Deserialize)]
pub(crate) struct ScriptsSection {
    disabled_hooks: Vec<usize>,
    disabled_handlers: Vec<usize>,
}

fn enc<T: Serialize>(v: &T) -> Vec<u8> {
    rmp_serde::to_vec_named(v).expect("save data is plain data")
}

fn dec<T: DeserializeOwned>(s: &Snapshot, name: &str) -> Result<T, String> {
    let bytes = s.sections.get(name).ok_or_else(|| format!("the save has no {name} section"))?;
    rmp_serde::from_slice(bytes).map_err(|e| format!("{name}: {e}"))
}

/// A 64-bit hash of canonical bytes (FNV-1a, then mixed).
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    let h = bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
    crate::rng::mix(h)
}

fn lock_matches(saved: &[(String, String)], loaded: &[crate::modloader::ModManifest]) -> bool {
    saved.len() == loaded.len() && saved.iter().zip(loaded).all(|(s, m)| s.0 == m.id && s.1 == m.version)
}

/// An event with its def ids mapped, or `None` if one no longer exists.
fn remap_event(r: &Remap, e: GameEvent) -> Option<GameEvent> {
    Some(match e {
        GameEvent::PawnJoined { id, name, def } => GameEvent::PawnJoined { id, name, def: r.get("creature", def)? },
        GameEvent::PawnDied { id, name, def, faction, pos, founder } => {
            GameEvent::PawnDied { id, name, def: r.get("creature", def)?, faction, pos, founder }
        }
        GameEvent::PawnLeft { id, name, def, faction } => {
            GameEvent::PawnLeft { id, name, def: r.get("creature", def)?, faction }
        }
        GameEvent::BuildingComplete { id, def, pos } => {
            GameEvent::BuildingComplete { id, def: r.get("thing", def)?, pos }
        }
        other => other,
    })
}

fn def_table(defs: &DefDb) -> DefsSection {
    let kind = |k: &str, n: usize| (k.to_string(), (0..n).map(|i| defs.id_of(k, i as DefId)).collect());
    BTreeMap::from([
        kind("terrain", defs.terrain.len()),
        kind("thing", defs.things.len()),
        kind("creature", defs.creatures.len()),
        kind("need", defs.needs.len()),
        kind("designation", defs.designations.len()),
        kind("field", defs.fields.len()),
    ])
}

/// Saved def ids onto the loaded mods' def ids, kind by kind, by qualified
/// id. The identity when the defs are the ones the snapshot was taken with.
struct Remap<'a> {
    saved: &'a DefsSection,
    to: Option<BTreeMap<&'a str, Vec<Option<DefId>>>>,
}

impl<'a> Remap<'a> {
    fn new(saved: &'a DefsSection, now: &DefsSection) -> Remap<'a> {
        if saved == now {
            return Remap { saved, to: None };
        }
        let to = saved
            .iter()
            .map(|(kind, ids)| {
                let here = now.get(kind);
                let map = ids
                    .iter()
                    .map(|id| here.and_then(|h| h.iter().position(|x| x == id)).map(|i| i as DefId))
                    .collect();
                (kind.as_str(), map)
            })
            .collect();
        Remap { saved, to: Some(to) }
    }

    fn get(&self, kind: &str, d: DefId) -> Option<DefId> {
        match &self.to {
            None => Some(d),
            Some(to) => to.get(kind)?.get(d as usize).copied().flatten(),
        }
    }

    /// The saved qualified id, for a report.
    fn name(&self, kind: &str, d: DefId) -> &str {
        self.saved.get(kind).and_then(|ids| ids.get(d as usize)).map_or("?", |s| s.as_str())
    }
}

/// Every entity's value of component `C`, in id order.
fn component<C: hecs::Component + Serialize + Clone>(w: &World) -> Vec<u8> {
    let mut rows: Vec<(Entity, C)> = w.ecs.query::<(Entity, &C)>().iter().map(|(e, c)| (e, c.clone())).collect();
    rows.sort_unstable_by_key(|r| r.0.id());
    enc(&rows)
}

impl Snapshot {
    /// The world between two ticks.
    pub fn capture(sim: &Sim) -> Snapshot {
        let w = &sim.world;
        let header = Header {
            format: FORMAT,
            engine: env!("CARGO_PKG_VERSION").to_string(),
            api: format!("{}.{}", crate::API_VERSION.0, crate::API_VERSION.1),
            seed: w.seed,
            tick: w.tick,
            mods: sim.mods.iter().map(|m| (m.id.clone(), m.version.clone())).collect(),
        };
        let mut reservations: Vec<(Entity, Entity)> = w.reservations.iter().map(|(&t, &h)| (t, h)).collect();
        reservations.sort_unstable_by_key(|r| r.0.id());
        let world = WorldSection {
            width: w.map.w,
            height: w.map.h,
            rng: w.rng.state(),
            next_entity: w.next_entity,
            wealth: w.wealth,
            colony_lost: w.colony_lost,
            reservations,
            messages: w.messages.clone(),
            recent_events: w.recent_events.iter().map(|(t, k, e, n)| (*t, k.to_string(), *e, n.clone())).collect(),
            events: w.events.clone(),
            data_versions: w.data_versions.clone(),
        };
        let (disabled_hooks, disabled_handlers) = sim.scripts.disabled();
        let mut sections = BTreeMap::from([
            ("engine:defs".to_string(), enc(&def_table(&w.defs))),
            ("engine:world".to_string(), enc(&world)),
            ("engine:map".to_string(), enc(&w.map.terrain)),
            ("engine:fields".to_string(), enc(&w.fields.saved(&w.map))),
            ("engine:scripts".to_string(), enc(&ScriptsSection { disabled_hooks, disabled_handlers })),
            ("engine:pawn".to_string(), component::<Pawn>(w)),
            ("engine:thing".to_string(), component::<Thing>(w)),
            ("engine:blueprint".to_string(), component::<Blueprint>(w)),
            ("engine:made_of".to_string(), component::<MadeOf>(w)),
            ("engine:owner".to_string(), component::<Owner>(w)),
            ("engine:designated".to_string(), component::<Designated>(w)),
            ("engine:regrow".to_string(), component::<Regrow>(w)),
        ]);
        // Script data, one section per mod: every key is "mod:key" (0062).
        let mut by_mod: BTreeMap<&str, BTreeMap<&str, &Data>> = BTreeMap::new();
        for (k, v) in &w.data {
            let (m, key) = match k.split_once(':') {
                Some((m, key)) if !m.is_empty() => (m, key),
                _ => ("", k.as_str()),
            };
            by_mod.entry(m).or_default().insert(key, v);
        }
        for (m, data) in by_mod {
            sections.insert(format!("{m}:data"), enc(&data));
        }
        Snapshot { header, sections }
    }

    /// The living colonists' names, the founder first: what a list of saves
    /// shows without loading the game.
    pub fn colonists(&self) -> Result<Vec<String>, String> {
        let mut pawns: Vec<(Entity, Pawn)> = dec(self, "engine:pawn")?;
        pawns.retain(|(_, p)| p.active && !p.dead && p.faction == Faction::Player);
        pawns.sort_by_key(|(e, p)| (!p.founder, e.id()));
        Ok(pawns.into_iter().map(|(_, p)| p.name).collect())
    }

    /// Hash of every section, in name order.
    pub fn hash(&self) -> u64 {
        self.sections.iter().fold(hash_bytes(&enc(&self.header)), |h, (name, bytes)| {
            crate::rng::mix(h ^ hash_bytes(name.as_bytes()) ^ hash_bytes(bytes).rotate_left(17))
        })
    }

    /// The file form: a header, then each section compressed on its own.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        let mut put = |b: &[u8]| {
            out.extend_from_slice(&(b.len() as u32).to_le_bytes());
            out.extend_from_slice(b);
        };
        put(&enc(&self.header));
        for (name, bytes) in &self.sections {
            put(name.as_bytes());
            put(&zstd::encode_all(bytes.as_slice(), 3).expect("compressing to memory"));
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Snapshot, String> {
        let rest = bytes.strip_prefix(MAGIC.as_slice()).ok_or("not a rim snapshot")?;
        let mut rest = rest;
        let mut take = || -> Result<Option<&[u8]>, String> {
            if rest.is_empty() {
                return Ok(None);
            }
            let (len, tail) = rest.split_first_chunk::<4>().ok_or("a truncated snapshot")?;
            let len = u32::from_le_bytes(*len) as usize;
            let (b, tail) = tail.split_at_checked(len).ok_or("a truncated snapshot")?;
            rest = tail;
            Ok(Some(b))
        };
        let header = take()?.ok_or("a snapshot with no header")?;
        let header: Header = rmp_serde::from_slice(header).map_err(|e| format!("header: {e}"))?;
        let mut sections = BTreeMap::new();
        while let Some(name) = take()? {
            let name = String::from_utf8(name.to_vec()).map_err(|_| "a section name isn't text")?;
            let packed = take()?.ok_or("a section with no body")?;
            let bytes = zstd::decode_all(packed).map_err(|e| format!("{name}: {e}"))?;
            sections.insert(name, bytes);
        }
        Ok(Snapshot { header, sections })
    }

    /// Load the mods and rebuild the world this snapshot holds. The defs must
    /// be the ones it was taken with, since its def ids index them; a mod
    /// list that differs without changing any def (a new engine, a script-only
    /// mod) loads. Deciding that a load starts a new epoch is the save
    /// file's job (DESIGN.md §7a).
    pub fn restore(&self, mods_dir: &Path, enabled: &dyn Fn(&str) -> bool) -> Result<Sim, String> {
        let (sim, notes) = self.restore_noting(mods_dir, enabled)?;
        match notes.is_empty() {
            true => Ok(sim),
            false => Err(format!("the save doesn't load as it was: {}", notes.join("; "))),
        }
    }

    /// `restore`, across a change of defs: every def reference is mapped by
    /// its qualified id onto the loaded mods'. What no longer exists is
    /// dropped (an entity whose def is gone, a material, a need) and each
    /// drop is noted. A removed mod's script data stays in the world, so it
    /// rides along in every later snapshot until the mod comes back.
    pub fn restore_noting(
        &self,
        mods_dir: &Path,
        enabled: &dyn Fn(&str) -> bool,
    ) -> Result<(Sim, Vec<String>), String> {
        if self.header.format != FORMAT {
            return Err(format!("save format {} (this build reads {FORMAT})", self.header.format));
        }
        let mods = Sim::load_mods(mods_dir, enabled)?;
        let defs = mods.defs.clone();
        let saved_defs: DefsSection = dec(self, "engine:defs")?;
        let remap = Remap::new(&saved_defs, &def_table(&defs));
        let mut notes: Vec<String> = Vec::new();
        let ws: WorldSection = dec(self, "engine:world")?;
        let mut w = World::new(defs.clone(), ws.width, ws.height, self.header.seed);
        w.tick = self.header.tick;
        w.rng = crate::rng::Rng::from_state(ws.rng);
        w.wealth = ws.wealth;
        w.colony_lost = ws.colony_lost;
        w.messages = ws.messages;
        let pending = ws.events.len();
        w.events = ws.events.into_iter().filter_map(|e| remap_event(&remap, e)).collect();
        if w.events.len() < pending {
            notes.push(format!("dropped {} pending events", pending - w.events.len()));
        }
        w.recent_events = ws
            .recent_events
            .into_iter()
            .map(|(t, k, e, n)| {
                let kind = match k.as_str() {
                    "joined" => "joined",
                    "left" => "left",
                    "died" => "died",
                    "founder_died" => "founder_died",
                    _ => "event",
                };
                (t, kind, e, n)
            })
            .collect();

        let terrain: Vec<DefId> = dec(self, "engine:map")?;
        if terrain.len() != (ws.width * ws.height) as usize {
            return Err("engine:map doesn't match the map's size".into());
        }
        let mut lost_terrain: BTreeMap<&str, u32> = BTreeMap::new();
        for (i, &t) in terrain.iter().enumerate() {
            let p = w.map.pos(i);
            // Ground from a removed mod becomes the first terrain there is.
            let now = remap.get("terrain", t).filter(|&d| (d as usize) < defs.terrain.len()).unwrap_or_else(|| {
                *lost_terrain.entry(remap.name("terrain", t)).or_default() += 1;
                0
            });
            w.map.set_terrain(p, now, defs.terrain[now as usize].path_cost);
        }
        for (id, n) in lost_terrain {
            notes.push(format!("{n} cells of {id} became {}", defs.terrain[0].id));
        }

        // Entities, in id order: each gets every component the save has for
        // it, with its def ids mapped. An entity whose def is gone is dropped.
        let mut builders: BTreeMap<u32, (Entity, EntityBuilder)> = BTreeMap::new();
        let mut gone: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        let mut dropped: BTreeMap<String, u32> = BTreeMap::new();
        let mut add = |e: Entity, c: &dyn Fn(&mut EntityBuilder)| {
            c(&mut builders.entry(e.id()).or_insert_with(|| (e, EntityBuilder::new())).1);
        };
        for (e, mut p) in dec::<Vec<(Entity, Pawn)>>(self, "engine:pawn")? {
            let Some(def) = remap.get("creature", p.def) else {
                gone.insert(e.id());
                *dropped.entry(format!("{} ({})", remap.name("creature", p.def), p.name)).or_default() += 1;
                continue;
            };
            p.def = def;
            let mut needs = Vec::new();
            for &(n, v) in &p.needs {
                match remap.get("need", n) {
                    Some(n) => needs.push((n, v)),
                    None => *dropped.entry(format!("need {}", remap.name("need", n))).or_default() += 1,
                }
            }
            p.needs = needs;
            if let Some((t, _)) = p.carry {
                match remap.get("thing", t) {
                    Some(now) => p.carry = p.carry.map(|(_, n)| (now, n)),
                    None => {
                        *dropped.entry(format!("carried {}", remap.name("thing", t))).or_default() += 1;
                        p.carry = None;
                    }
                }
            }
            if let Job::Comfort { need, .. } = &mut p.job {
                match remap.get("need", *need) {
                    Some(n) => *need = n,
                    None => p.job = Job::Idle,
                }
            }
            add(e, &|b| {
                b.add(p.clone());
            });
        }
        for (e, mut t) in dec::<Vec<(Entity, Thing)>>(self, "engine:thing")? {
            let Some(def) = remap.get("thing", t.def) else {
                gone.insert(e.id());
                *dropped.entry(remap.name("thing", t.def).to_string()).or_default() += 1;
                continue;
            };
            t.def = def;
            add(e, &|b| {
                b.add(t.clone());
            });
        }
        for (e, mut bp) in dec::<Vec<(Entity, Blueprint)>>(self, "engine:blueprint")? {
            let cost: Option<Vec<(DefId, u32)>> =
                bp.cost.iter().map(|&(t, n)| Some((remap.get("thing", t)?, n))).collect();
            match cost {
                Some(c) if !gone.contains(&e.id()) => {
                    bp.cost = c;
                    add(e, &|b| {
                        b.add(bp.clone());
                    });
                }
                _ => {
                    // A plan for something that needs a removed material.
                    gone.insert(e.id());
                    *dropped.entry("a blueprint".into()).or_default() += 1;
                }
            }
        }
        for (e, m) in dec::<Vec<(Entity, MadeOf)>>(self, "engine:made_of")? {
            match remap.get("thing", m.0) {
                Some(d) => add(e, &|b| {
                    b.add(MadeOf(d));
                }),
                // The thing stays, built of nothing in particular now.
                None if !gone.contains(&e.id()) => {
                    *dropped.entry(format!("material {}", remap.name("thing", m.0))).or_default() += 1
                }
                None => {}
            }
        }
        for (e, d) in dec::<Vec<(Entity, Designated)>>(self, "engine:designated")? {
            match remap.get("designation", d.0) {
                Some(d) => add(e, &|b| {
                    b.add(Designated(d));
                }),
                None if !gone.contains(&e.id()) => {
                    *dropped.entry(format!("designation {}", remap.name("designation", d.0))).or_default() += 1
                }
                None => {}
            }
        }
        for (e, o) in dec::<Vec<(Entity, Owner)>>(self, "engine:owner")? {
            add(e, &|b| {
                b.add(o);
            });
        }
        for (e, r) in dec::<Vec<(Entity, Regrow)>>(self, "engine:regrow")? {
            add(e, &|b| {
                b.add(r);
            });
        }
        for (id, n) in dropped {
            notes.push(if n == 1 { format!("dropped {id}") } else { format!("dropped {n} × {id}") });
        }
        for (id, (e, mut b)) in builders {
            if !gone.contains(&id) {
                w.ecs.spawn_at(e, b.build());
            }
        }
        w.reservations =
            ws.reservations.into_iter().filter(|(t, h)| !gone.contains(&t.id()) && !gone.contains(&h.id())).collect();
        w.next_entity = ws.next_entity;
        w.pawns = w.ecs.query::<(Entity, &Pawn)>().iter().map(|(e, _)| e).collect();
        w.pawns.sort_unstable_by_key(|e| e.id());

        // The map's entity layers, then field stamps once every wall is up.
        let mut things: Vec<(Entity, Thing, bool, Option<Faction>)> = w
            .ecs
            .query::<(Entity, &Thing, Option<&Blueprint>, Option<&Owner>)>()
            .iter()
            .map(|(e, t, bp, o)| (e, t.clone(), bp.is_some(), o.map(|o| o.0)))
            .collect();
        things.sort_unstable_by_key(|t| t.0.id());
        for (e, t, blueprint, owner) in &things {
            let td = defs.thing(t.def);
            match td.category {
                Category::Item => w.map.set_item(t.pos, Some(*e)),
                Category::Floor => w.map.set_floor(t.pos, Some(*e), if *blueprint { 0 } else { td.path_cost }),
                _ => {
                    let (blocks, cost, door) =
                        if *blueprint { (false, 0, false) } else { (td.blocks, td.path_cost, td.door) };
                    w.map.set_fixture(t.pos, Some(*e), blocks, cost, door);
                    w.map.set_owner(t.pos, *owner);
                }
            }
        }
        w.map.ensure_regions();
        w.map.ensure_rooms();
        for (e, t, blueprint, _) in &things {
            if !blueprint {
                w.fields.add_emitters(&defs, &w.map, *e, t.def, t.pos);
            }
        }
        // Stamps were made against the finished map: nothing to redo.
        w.map.take_changed_cells();
        let mut fields: SavedFields = dec(self, "engine:fields")?;
        if remap.to.is_some() {
            // Field state is kept per field: take each loaded field's from the
            // save by its id, and start one the save didn't have fresh.
            let fresh = w.fields.saved(&w.map);
            let old = fields.clone();
            let saved_fields = remap.saved.get("field").map_or(&[][..], |v| v.as_slice());
            let pick = |j: usize| {
                let i = saved_fields.iter().position(|id| *id == defs.fields[j].id);
                i.filter(|&i| i < old.atmos.len() && i < old.ambient.len() && i < old.rooms.len())
            };
            let n = defs.fields.len();
            fields.atmos = (0..n).map(|j| pick(j).map_or(fresh.atmos[j].clone(), |i| old.atmos[i].clone())).collect();
            fields.ambient = (0..n).map(|j| pick(j).map_or(fresh.ambient[j], |i| old.ambient[i])).collect();
            fields.rooms = (0..n).map(|j| pick(j).map_or(fresh.rooms[j].clone(), |i| old.rooms[i].clone())).collect();
        }
        // Under other defs the map itself may differ (a removed mod's walls
        // are gone), so room values are carried over cell by cell.
        w.fields.restore(&mut w.map, fields, remap.to.is_some());
        w.refresh_boundaries();

        for (name, bytes) in &self.sections {
            let Some(m) = name.strip_suffix(":data") else { continue };
            let data: BTreeMap<String, Data> = rmp_serde::from_slice(bytes).map_err(|e| format!("{name}: {e}"))?;
            w.data.extend(data.into_iter().map(|(k, v)| (if m.is_empty() { k } else { format!("{m}:{k}") }, v)));
        }
        // A mod whose version changed upgrades its data before anything runs
        // (0139); so does one coming back with a different version than its
        // parked data. Any failure fails the load: nothing is half-migrated.
        let mut written_by = ws.data_versions;
        written_by.extend(self.header.mods.iter().cloned());
        for m in &mods.manifests {
            let Some(from) = written_by.get(&m.id) else { continue };
            if *from != m.version {
                mods.scripts.migrate(&mut w, &m.id, from).map_err(|e| {
                    format!("mod '{}' couldn't upgrade its data from {from} to {}: {e}", m.id, m.version)
                })?;
            }
        }
        // Remember the version of every mod whose data is parked here.
        let loaded: std::collections::BTreeSet<&str> = mods.manifests.iter().map(|m| m.id.as_str()).collect();
        for k in w.data.keys() {
            let Some((m, _)) = k.split_once(':') else { continue };
            if let (false, Some(v)) = (loaded.contains(m), written_by.get(m)) {
                w.data_versions.insert(m.to_string(), v.clone());
            }
        }
        let sc: ScriptsSection = dec(self, "engine:scripts")?;
        // Hook indices only mean the same hooks under the same scripts.
        if remap.to.is_none() && lock_matches(&self.header.mods, &mods.manifests) {
            mods.scripts.set_disabled(&sc.disabled_hooks, &sc.disabled_handlers);
        }
        Ok((Sim::assemble(mods, w), notes))
    }
}
