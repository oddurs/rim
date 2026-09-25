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
struct WorldSection {
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
}

/// Each def kind's qualified ids, in `DefId` order: the table the raw ids in
/// every other section index into.
type DefsSection = BTreeMap<String, Vec<String>>;

#[derive(Serialize, Deserialize)]
struct ScriptsSection {
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
        if self.header.format != FORMAT {
            return Err(format!("save format {} (this build reads {FORMAT})", self.header.format));
        }
        let mods = Sim::load_mods(mods_dir, enabled)?;
        let defs = mods.defs.clone();
        if dec::<DefsSection>(self, "engine:defs")? != def_table(&defs) {
            let loaded: Vec<&str> = mods.manifests.iter().map(|m| m.id.as_str()).collect();
            return Err(format!(
                "the mods' defs differ from the save's (saved with {:?}, loaded {loaded:?}); \
                 loading across a def change needs a migration",
                self.header.mods.iter().map(|m| m.0.as_str()).collect::<Vec<_>>()
            ));
        }
        let ws: WorldSection = dec(self, "engine:world")?;
        let mut w = World::new(defs.clone(), ws.width, ws.height, self.header.seed);
        w.tick = self.header.tick;
        w.rng = crate::rng::Rng::from_state(ws.rng);
        w.wealth = ws.wealth;
        w.colony_lost = ws.colony_lost;
        w.reservations = ws.reservations.into_iter().collect();
        w.messages = ws.messages;
        w.events = ws.events;
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
        for (i, &t) in terrain.iter().enumerate() {
            let p = w.map.pos(i);
            w.map.set_terrain(p, t, defs.terrain[t as usize].path_cost);
        }

        // Entities, in id order: each gets every component the save has for it.
        let mut builders: BTreeMap<u32, (Entity, EntityBuilder)> = BTreeMap::new();
        fn add<C: hecs::Component + DeserializeOwned>(
            s: &Snapshot,
            name: &str,
            b: &mut BTreeMap<u32, (Entity, EntityBuilder)>,
        ) -> Result<(), String> {
            for (e, c) in dec::<Vec<(Entity, C)>>(s, name)? {
                b.entry(e.id()).or_insert_with(|| (e, EntityBuilder::new())).1.add(c);
            }
            Ok(())
        }
        add::<Pawn>(self, "engine:pawn", &mut builders)?;
        add::<Thing>(self, "engine:thing", &mut builders)?;
        add::<Blueprint>(self, "engine:blueprint", &mut builders)?;
        add::<MadeOf>(self, "engine:made_of", &mut builders)?;
        add::<Owner>(self, "engine:owner", &mut builders)?;
        add::<Designated>(self, "engine:designated", &mut builders)?;
        add::<Regrow>(self, "engine:regrow", &mut builders)?;
        for (_, (e, mut b)) in builders {
            w.ecs.spawn_at(e, b.build());
        }
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
        let fields: SavedFields = dec(self, "engine:fields")?;
        w.fields.restore(&mut w.map, fields);
        w.refresh_boundaries();

        for (name, bytes) in &self.sections {
            let Some(m) = name.strip_suffix(":data") else { continue };
            let data: BTreeMap<String, Data> = rmp_serde::from_slice(bytes).map_err(|e| format!("{name}: {e}"))?;
            w.data.extend(data.into_iter().map(|(k, v)| (if m.is_empty() { k } else { format!("{m}:{k}") }, v)));
        }
        let sc: ScriptsSection = dec(self, "engine:scripts")?;
        mods.scripts.set_disabled(&sc.disabled_hooks, &sc.disabled_handlers);
        Ok(Sim::assemble(mods, w))
    }
}
