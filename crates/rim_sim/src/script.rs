//! Luau plugin host.
//!
//! Scripts run once at load to register hooks (`rim.every`) and event
//! handlers (`rim.on`). The world-facing API (`rim.spawn_pawn`, `rim.wealth`,
//! ...) only works inside those callbacks. It draws from the world's RNG, and
//! `math.random`/`os` are removed, so scripts stay deterministic.
//!
//! Each script gets its own global environment (reads fall through to the
//! shared globals), so mods can't clobber each other by accident. The `rim`
//! table itself is shared: that's how one plugin offers an API to others
//! (e.g. core's storyteller exposes `rim.register_incident`).

use crate::modloader::ScriptSource;
use crate::path::Goal;
use crate::profile::Profile;
use crate::world::*;
use crate::{IVec, TICKS_PER_DAY};
use mlua::{Function, IntoLuaMulti, Lua, Table, Value};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

struct WorldPtr(Cell<*mut World>);

/// Run `f` against the world that is lent to the current hook call.
fn with_world<R>(ptr: &WorldPtr, f: impl FnOnce(&mut World) -> mlua::Result<R>) -> mlua::Result<R> {
    let p = ptr.0.get();
    if p.is_null() {
        return Err(mlua::Error::runtime("world API is only available inside rim.every / rim.on callbacks"));
    }
    // SAFETY: the pointer is set only for the duration of `ScriptHost::call`,
    // during which the host touches the World through nothing else.
    f(unsafe { &mut *p })
}

fn rim_sim_entity(id: u64) -> mlua::Result<hecs::Entity> {
    hecs::Entity::from_bits(id).ok_or_else(|| mlua::Error::runtime(format!("bad entity id {id}")))
}

struct Hook {
    mod_id: String,
    interval: u64,
    phase: u64,
    func: Function,
}

struct Handler {
    mod_id: String,
    event: String,
    func: Function,
}

#[derive(Default)]
struct Registry {
    current_mod: String,
    hooks: Vec<Hook>,
    handlers: Vec<Handler>,
}

pub struct ScriptHost {
    lua: Lua,
    world: Rc<WorldPtr>,
    reg: Rc<RefCell<Registry>>,
}

impl ScriptHost {
    pub fn load(scripts: &[ScriptSource], defs: &crate::defs::DefDb) -> Result<Self, String> {
        let host = ScriptHost {
            lua: Lua::new(),
            world: Rc::new(WorldPtr(Cell::new(std::ptr::null_mut()))),
            reg: Rc::new(RefCell::new(Registry::default())),
        };
        host.install(defs).map_err(|e| format!("script API setup failed: {e}"))?;
        for s in scripts {
            host.reg.borrow_mut().current_mod = s.mod_id.clone();
            let name = format!("{}/scripts/{}", s.mod_id, s.name);
            let env = host.env().map_err(|e| e.to_string())?;
            host.lua
                .load(&s.source)
                .set_name(format!("@{name}"))
                .set_environment(env)
                .exec()
                .map_err(|e| format!("{name}: {e}"))?;
        }
        Ok(host)
    }

    fn env(&self) -> mlua::Result<Table> {
        let env = self.lua.create_table()?;
        let mt = self.lua.create_table()?;
        mt.set("__index", self.lua.globals())?;
        env.set_metatable(Some(mt));
        Ok(env)
    }

    fn install(&self, defs: &crate::defs::DefDb) -> mlua::Result<()> {
        let lua = &self.lua;
        let g = lua.globals();
        g.set("os", Value::Nil)?;
        let math: Table = g.get("math")?;
        math.set("random", Value::Nil)?;
        math.set("randomseed", Value::Nil)?;

        let rim = lua.create_table()?;
        rim.set("api_version", format!("{}.{}", crate::API_VERSION.0, crate::API_VERSION.1))?;
        rim.set("ticks_per_day", TICKS_PER_DAY)?;

        // ---- static data: defs are known at load time
        let creatures = lua.create_table()?;
        for cd in &defs.creatures {
            let t = lua.create_table()?;
            t.set("id", cd.id.as_str())?;
            t.set("label", cd.label.as_str())?;
            t.set("intelligent", cd.intelligent)?;
            t.set("aggressive", cd.aggressive)?;
            t.set("flees", cd.flees)?;
            t.set("plural", cd.plural.as_str())?;
            t.set("market_value", cd.market_value)?;
            t.set("max_hp", cd.max_hp)?;
            t.set("wild", cd.spawn.is_some())?;
            creatures.push(t)?;
        }
        rim.set("creature_defs", creatures)?;
        let things = lua.create_table()?;
        for td in &defs.things {
            let t = lua.create_table()?;
            t.set("id", td.id.as_str())?;
            t.set("label", td.label.as_str())?;
            t.set("market_value", td.market_value)?;
            t.set("food", td.food.is_some())?;
            t.set("item", td.category == crate::defs::Category::Item)?;
            things.push(t)?;
        }
        rim.set("thing_defs", things)?;

        // ---- registration
        let reg = self.reg.clone();
        rim.set(
            "every",
            lua.create_function(move |_, (interval, func): (u64, Function)| {
                let mut r = reg.borrow_mut();
                let interval = interval.max(1);
                // Stagger hooks so they don't all land on the same tick.
                let phase = (r.hooks.len() as u64 * 37) % interval;
                let mod_id = r.current_mod.clone();
                r.hooks.push(Hook { mod_id, interval, phase, func });
                Ok(())
            })?,
        )?;
        let reg = self.reg.clone();
        rim.set(
            "on",
            lua.create_function(move |_, (event, func): (String, Function)| {
                let mut r = reg.borrow_mut();
                let mod_id = r.current_mod.clone();
                r.handlers.push(Handler { mod_id, event, func });
                Ok(())
            })?,
        )?;
        let reg = self.reg.clone();
        rim.set(
            "log",
            lua.create_function(move |_, msg: String| {
                eprintln!("[{}] {msg}", reg.borrow().current_mod);
                Ok(())
            })?,
        )?;

        // ---- world API (only valid inside callbacks)
        macro_rules! api {
            ($name:literal, $ty:ty, |$w:ident, $args:pat_param| $body:expr) => {{
                let ptr = self.world.clone();
                rim.set($name, lua.create_function(move |_, $args: $ty| with_world(&ptr, |$w| $body))?)?;
            }};
        }

        api!("tick", (), |w, _a| Ok(w.tick));
        api!("day", (), |w, _a| Ok(w.day()));
        api!("hour", (), |w, _a| Ok(w.hour()));
        api!("wealth", (), |w, _a| Ok(w.wealth));
        api!("map_size", (), |w, _a| Ok((w.map.w, w.map.h)));
        api!("random", (), |w, _a| Ok(w.rng.float()));
        api!("random_int", (i32, i32), |w, (a, b)| Ok(w.rng.range(a, b)));
        api!("colonists", (), |w, _a| Ok(w.colonists().count()));
        api!("colony_center", (), |w, _a| Ok(match w.colony_center() {
            Some(c) => (Some(c.x), Some(c.y)),
            None => (None, None),
        }));
        api!("count_pawns", String, |w, faction| {
            let f =
                Faction::parse(&faction).ok_or_else(|| mlua::Error::runtime(format!("unknown faction '{faction}'")))?;
            Ok(w.pawns
                .iter()
                .filter(|&&e| w.ecs.get::<&Pawn>(e).is_ok_and(|p| p.active && !p.dead && p.faction == f))
                .count())
        });
        // Rough melee output of the colony: what raids are weighed against.
        api!("colony_strength", (), |w, _a| {
            let defs = w.defs.clone();
            let mut s = 0.0;
            for e in w.colonists() {
                if let Ok(p) = w.ecs.get::<&Pawn>(e) {
                    let cd = defs.creature(p.def);
                    s += (p.hp as f64 / cd.max_hp as f64) * cd.melee_damage as f64 * 60.0 / cd.melee_cooldown as f64;
                }
            }
            Ok(s)
        });
        api!("message", (String, Option<String>), |w, (text, kind)| {
            w.message(text, MsgKind::parse(kind.as_deref().unwrap_or("info")));
            Ok(())
        });
        // A random open cell on the map edge that can reach the colony.
        api!("edge_cell", (), |w, _a| {
            w.map.ensure_regions();
            let center = w.colony_center();
            let (mw, mh) = (w.map.w, w.map.h);
            for _ in 0..400 {
                let t = w.rng.below(mw.max(mh) as u32) as i32;
                let p = match w.rng.below(4) {
                    0 => IVec::new(t.min(mw - 1), 0),
                    1 => IVec::new(t.min(mw - 1), mh - 1),
                    2 => IVec::new(0, t.min(mh - 1)),
                    _ => IVec::new(mw - 1, t.min(mh - 1)),
                };
                if w.map.passable(p) && center.is_none_or(|c| w.map.can_reach(p, Goal::Cell(c))) {
                    return Ok((Some(p.x), Some(p.y)));
                }
            }
            Ok((None, None))
        });
        // A random open cell within `r` of (x, y).
        api!("near_cell", (i32, i32, i32), |w, (x, y, r)| {
            for _ in 0..100 {
                let p = IVec::new(x + w.rng.range(-r, r), y + w.rng.range(-r, r));
                if w.map.passable(p) {
                    return Ok((Some(p.x), Some(p.y)));
                }
            }
            Ok((None, None))
        });
        api!("spawn_pawn", (String, String, i32, i32, Option<String>), |w, (creature, faction, x, y, name)| {
            let def = w
                .defs
                .creature_id(&creature)
                .ok_or_else(|| mlua::Error::runtime(format!("unknown creature '{creature}'")))?;
            let f =
                Faction::parse(&faction).ok_or_else(|| mlua::Error::runtime(format!("unknown faction '{faction}'")))?;
            let p = IVec::new(x, y);
            if !w.map.passable(p) {
                return Ok((None, None));
            }
            let e = w.spawn_pawn(def, f, p, name);
            let name = w.ecs.get::<&Pawn>(e).map(|p| p.name.clone()).unwrap_or_default();
            Ok((Some(e.to_bits().get()), Some(name)))
        });
        // Make a pawn give up and walk off the map after `ticks`.
        api!("leave_after", (u64, u64), |w, (id, ticks)| {
            let e = rim_sim_entity(id)?;
            let t = w.tick + ticks;
            if let Ok(mut p) = w.ecs.get::<&mut Pawn>(e) {
                p.leave_at = Some(t);
            }
            Ok(())
        });
        api!("spawn_item", (String, i32, i32, u32), |w, (thing, x, y, count)| {
            let def =
                w.defs.thing_id(&thing).ok_or_else(|| mlua::Error::runtime(format!("unknown thing '{thing}'")))?;
            Ok(w.place_item(def, IVec::new(x, y), count))
        });

        g.set("rim", rim)?;
        Ok(())
    }

    fn call(&self, w: &mut World, prof: &mut Profile, mod_id: &str, f: &Function, args: impl IntoLuaMulti) {
        self.world.0.set(w as *mut World);
        let t = Instant::now();
        let r = f.call::<()>(args);
        self.world.0.set(std::ptr::null_mut());
        prof.add(&format!("mod:{mod_id}"), t.elapsed().as_secs_f64() * 1e6);
        if let Err(e) = r {
            let text = format!("[{mod_id}] script error: {e}");
            eprintln!("{text}");
            w.message(text, MsgKind::Bad);
        }
    }

    pub fn run_hooks(&self, w: &mut World, prof: &mut Profile) {
        let due: Vec<(Function, String)> = {
            let r = self.reg.borrow();
            r.hooks
                .iter()
                .filter(|h| (w.tick + h.phase).is_multiple_of(h.interval))
                .map(|h| (h.func.clone(), h.mod_id.clone()))
                .collect()
        };
        for (f, m) in due {
            self.call(w, prof, &m, &f, ());
        }
    }

    pub fn dispatch_events(&self, w: &mut World, prof: &mut Profile) {
        if w.events.is_empty() {
            return;
        }
        let events = std::mem::take(&mut w.events);
        for ev in events {
            let (name, t) = match self.event_table(w, &ev) {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("event conversion failed: {e}");
                    continue;
                }
            };
            let targets: Vec<(Function, String)> = {
                let r = self.reg.borrow();
                r.handlers.iter().filter(|h| h.event == name).map(|h| (h.func.clone(), h.mod_id.clone())).collect()
            };
            for (f, m) in targets {
                self.call(w, prof, &m, &f, t.clone());
            }
        }
    }

    fn event_table(&self, w: &World, ev: &GameEvent) -> mlua::Result<(&'static str, Table)> {
        let t = self.lua.create_table()?;
        let defs = &w.defs;
        let name = match ev {
            GameEvent::PawnJoined { id, name, def } => {
                t.set("id", id.to_bits().get())?;
                t.set("name", name.as_str())?;
                t.set("creature", defs.creature(*def).id.as_str())?;
                "pawn_joined"
            }
            GameEvent::PawnDied { id, name, def, faction, pos } => {
                t.set("id", id.to_bits().get())?;
                t.set("name", name.as_str())?;
                t.set("creature", defs.creature(*def).id.as_str())?;
                t.set("faction", faction.name())?;
                t.set("x", pos.x)?;
                t.set("y", pos.y)?;
                "pawn_died"
            }
            GameEvent::PawnLeft { id, name, def, faction } => {
                t.set("id", id.to_bits().get())?;
                t.set("name", name.as_str())?;
                t.set("creature", defs.creature(*def).id.as_str())?;
                t.set("faction", faction.name())?;
                "pawn_left"
            }
            GameEvent::BuildingComplete { id, def, pos } => {
                t.set("id", id.to_bits().get())?;
                t.set("thing", defs.thing(*def).id.as_str())?;
                t.set("x", pos.x)?;
                t.set("y", pos.y)?;
                "building_complete"
            }
            GameEvent::NewDay { day } => {
                t.set("day", *day)?;
                "new_day"
            }
            GameEvent::ColonyLost => "colony_lost",
        };
        Ok((name, t))
    }

    pub fn hook_count(&self) -> (usize, usize) {
        let r = self.reg.borrow();
        (r.hooks.len(), r.handlers.len())
    }
}
