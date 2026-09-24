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
use mlua::chunk::Compiler;
use mlua::{Function, IntoLuaMulti, Lua, LuaOptions, StdLib, Table, Value, VmState};
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

fn field_id(w: &World, id: &str) -> mlua::Result<usize> {
    w.defs.lookup("field", id).map(|f| f as usize).ok_or_else(|| mlua::Error::runtime(format!("unknown field '{id}'")))
}

/// The mod whose code is calling into the engine: the chunk name of the
/// nearest Luau frame ("@weather/scripts/00_weather.luau" is weather's). Not
/// the mod whose hook is running: when mod B calls `rim.weather.force`, it's
/// the weather plugin's code that emits `weather:changed`.
fn calling_mod(lua: &Lua) -> Option<String> {
    (1..16).find_map(|level| {
        lua.inspect_stack(level, |d| {
            let src = d.source().source?.to_string();
            let rest = src.strip_prefix('@')?;
            let (mod_id, path) = rest.split_once('/')?;
            path.starts_with("scripts/").then(|| mod_id.to_string())
        })
        .flatten()
    })
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
    /// The mod whose code is running: loading, or in a hook or handler.
    current_mod: String,
    /// Who put each key in `rim` ("the engine", or "mod 'x'"): nothing may be
    /// replaced, only added, and only while mods load.
    owners: std::collections::HashMap<String, String>,
    /// Set once every script has loaded: `rim` is read-only from then on.
    loaded: bool,
    /// Hooks and handlers stopped for running away (by function pointer).
    disabled: std::collections::HashSet<usize>,
    hooks: Vec<Hook>,
    handlers: Vec<Handler>,
}

/// Most memory the sim VM may hold. Hitting it fails the allocating script
/// with an error, like any other script error, instead of taking the game down.
pub const MEMORY_LIMIT: usize = 256 << 20;

/// Most interrupts (loop back-edges and calls) one hook or handler call may
/// run before it's stopped as a runaway. Counted, not timed: a wall-clock
/// limit would stop peers at different points and desync them. Generous: the
/// weather plugin's heaviest call uses a few thousand.
pub const STEP_BUDGET: u64 = 100_000_000;

/// Average time a mod's script calls may take before the profiler warns
/// about it (µs). A whole tick at 6x has 2 ms (DESIGN.md §8).
pub const MOD_BUDGET_US: f64 = 500.0;

pub struct ScriptHost {
    /// Load-time advice for mod authors (determinism hazards).
    pub warnings: Vec<String>,
    lua: Lua,
    world: Rc<WorldPtr>,
    reg: Rc<RefCell<Registry>>,
    /// Interrupts left for the current call (see `STEP_BUDGET`).
    steps: Rc<Cell<u64>>,
    /// The current call used up its step budget.
    ran_away: Rc<Cell<bool>>,
}

/// Only libraries whose results are the same on every machine and that can't
/// reach outside the game: no os, io, debug or coroutine.
fn sim_libs() -> StdLib {
    StdLib::MATH | StdLib::STRING | StdLib::TABLE | StdLib::BIT | StdLib::UTF8 | StdLib::BUFFER
}

/// Globals removed from the sim VM. `collectgarbage` and `gcinfo` report
/// memory, which differs between machines; `loadstring` compiles code at run
/// time; `getfenv`/`setfenv` reach other mods' environments and turn off
/// Luau's fast paths; `math.random` is replaced by `rim.random` (the world RNG).
const REMOVED: &[&str] = &["collectgarbage", "gcinfo", "loadstring", "getfenv", "setfenv", "newproxy", "os"];

/// `math` functions whose C library versions differ between platforms in the
/// last bit. The sim VM replaces them with the `libm` crate's (a Rust port of
/// musl's): the same code, so the same bits, on every machine. They're also
/// disabled as compiler builtins, or Luau would constant-fold them with the
/// compiling machine's library and fast-call the C versions directly.
const LIBM: &[&str] =
    &["sin", "cos", "tan", "asin", "acos", "atan", "atan2", "exp", "log", "log10", "pow", "sinh", "cosh", "tanh"];

/// Lines where a script uses `^` with an exponent Luau hands to the
/// platform's `pow` (anything but a literal 2, 3 or 0.5, which it computes
/// exactly). Strings and comments are skipped.
pub fn pow_operator_lines(src: &str) -> Vec<usize> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let (mut i, mut line) = (0, 1);
    // `[[`, `[=[`... long brackets: returns the closing `]==]` if one opens here.
    let long_close = |i: usize| -> Option<Vec<u8>> {
        if b.get(i) != Some(&b'[') {
            return None;
        }
        let mut j = i + 1;
        while b.get(j) == Some(&b'=') {
            j += 1;
        }
        (b.get(j) == Some(&b'[')).then(|| {
            let mut c = vec![b']'];
            c.extend(std::iter::repeat_n(b'=', j - i - 1));
            c.push(b']');
            c
        })
    };
    while i < b.len() {
        match b[i] {
            b'\n' => line += 1,
            b'-' if b.get(i + 1) == Some(&b'-') => {
                i += 2;
                if let Some(close) = long_close(i) {
                    while i < b.len() && !b[i..].starts_with(&close) {
                        line += (b[i] == b'\n') as usize;
                        i += 1;
                    }
                    i += close.len();
                } else {
                    while i < b.len() && b[i] != b'\n' {
                        i += 1;
                    }
                }
                continue;
            }
            b'[' if long_close(i).is_some() => {
                let close = long_close(i).unwrap();
                while i < b.len() && !b[i..].starts_with(&close) {
                    line += (b[i] == b'\n') as usize;
                    i += 1;
                }
                i += close.len();
                continue;
            }
            q @ (b'"' | b'\'' | b'`') => {
                i += 1;
                while i < b.len() && b[i] != q {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    line += (b.get(i) == Some(&b'\n')) as usize;
                    i += 1;
                }
            }
            b'^' => {
                let rest = src[i + 1..].trim_start_matches([' ', '\t']);
                let lit: String =
                    rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '_').collect();
                if !matches!(lit.as_str(), "2" | "3" | "0.5" | "2.0" | "3.0") && !out.contains(&line) {
                    out.push(line);
                }
            }
            _ => {}
        }
        i += 1;
    }
    out
}

impl ScriptHost {
    pub fn load(scripts: &[ScriptSource], defs: &crate::defs::DefDb) -> Result<Self, String> {
        let lua = Lua::new_with(sim_libs(), LuaOptions::default()).map_err(|e| format!("script VM: {e}"))?;
        // Level 2 inlines small local functions and unrolls constant loops;
        // debug level 1 keeps line numbers in errors.
        lua.set_compiler(
            Compiler::new()
                .set_optimization_level(2)
                .set_debug_level(1)
                .set_disabled_builtins(LIBM.iter().map(|f| format!("math.{f}"))),
        );
        lua.set_memory_limit(MEMORY_LIMIT).map_err(|e| format!("script VM: {e}"))?;
        let steps = Rc::new(Cell::new(STEP_BUDGET));
        let ran_away = Rc::new(Cell::new(false));
        {
            let steps = steps.clone();
            let ran_away = ran_away.clone();
            lua.set_interrupt(move |_| {
                let left = steps.get();
                if left == 0 {
                    ran_away.set(true);
                    return Err(mlua::Error::runtime(format!(
                        "stopped: ran more than {STEP_BUDGET} steps in one call (an endless loop?)"
                    )));
                }
                steps.set(left - 1);
                Ok(VmState::Continue)
            });
        }
        let mut host = ScriptHost {
            warnings: Vec::new(),
            lua,
            world: Rc::new(WorldPtr(Cell::new(std::ptr::null_mut()))),
            reg: Rc::new(RefCell::new(Registry::default())),
            steps,
            ran_away,
        };
        host.install(defs).map_err(|e| format!("script API setup failed: {e}"))?;
        host.lock_down().map_err(|e| format!("script API setup failed: {e}"))?;
        for s in scripts {
            for line in pow_operator_lines(&s.source) {
                host.warnings.push(format!(
                    "{}/scripts/{}:{line}: `^` uses the platform's pow, which can differ between machines and desync \
                     co-op; use math.pow (deterministic), or x*x",
                    s.mod_id, s.name
                ));
            }
            host.reg.borrow_mut().current_mod = s.mod_id.clone();
            host.steps.set(STEP_BUDGET);
            let name = format!("{}/scripts/{}", s.mod_id, s.name);
            let env = host.env().map_err(|e| e.to_string())?;
            host.lua
                .load(&s.source)
                .set_name(format!("@{name}"))
                .set_environment(env)
                .exec()
                .map_err(|e| format!("{name}: {e}"))?;
        }
        host.freeze_api().map_err(|e| format!("script API setup failed: {e}"))?;
        Ok(host)
    }

    /// Every script has loaded: `rim` and every table in it (the engine's
    /// and each plugin's API) become read-only, so no mod can change another
    /// mod's or the engine's API while the game runs.
    fn freeze_api(&self) -> mlua::Result<()> {
        let mut r = self.reg.borrow_mut();
        r.loaded = true;
        r.current_mod.clear();
        drop(r);
        let api: Table = self.lua.named_registry_value("rim_api")?;
        for pair in api.pairs::<Value, Value>() {
            if let (_, Value::Table(t)) = pair? {
                t.set_readonly(true);
            }
        }
        api.set_readonly(true);
        Ok(())
    }

    /// A script's own global environment: its globals land here, and reads
    /// fall through to the shared globals.
    ///
    /// Marked safe so Luau takes its fast paths (cached imports like
    /// `math.floor`, builtin fastcalls, fast `pairs`); without it every global
    /// access is a full lookup. The price: a chain like `rim.weather.register`
    /// is resolved when the script loads, so replacing a function in `rim`
    /// later isn't seen by scripts loaded before. Plugins add to `rim`; they
    /// don't monkey-patch it.
    fn env(&self) -> mlua::Result<Table> {
        let env = self.lua.create_table()?;
        let mt = self.lua.create_table()?;
        mt.set("__index", self.lua.globals())?;
        env.set_metatable(Some(mt))?;
        env.set_safeenv(true);
        Ok(env)
    }

    /// Make the standard libraries and the global table read-only, so no mod
    /// can break another's `math` or `string`. `rim` itself stays writable:
    /// that's how plugins offer APIs to each other.
    fn lock_down(&self) -> mlua::Result<()> {
        let g = self.lua.globals();
        for lib in ["math", "string", "table", "bit32", "utf8", "buffer"] {
            if let Ok(Value::Table(t)) = g.get::<Value>(lib) {
                t.set_readonly(true);
            }
        }
        g.set_readonly(true);
        Ok(())
    }

    fn install(&self, defs: &crate::defs::DefDb) -> mlua::Result<()> {
        let lua = &self.lua;
        let g = lua.globals();
        for name in REMOVED {
            g.set(*name, Value::Nil)?;
        }
        let math: Table = g.get("math")?;
        // Deterministic replacements for the C library's transcendentals.
        macro_rules! libm1 {
            ($($name:literal => $f:path),*) => {$(
                math.set($name, lua.create_function(|_, x: f64| Ok($f(x)))?)?;
            )*};
        }
        libm1!(
            "sin" => libm::sin, "cos" => libm::cos, "tan" => libm::tan,
            "asin" => libm::asin, "acos" => libm::acos, "atan" => libm::atan,
            "exp" => libm::exp, "log10" => libm::log10,
            "sinh" => libm::sinh, "cosh" => libm::cosh, "tanh" => libm::tanh
        );
        math.set("atan2", lua.create_function(|_, (y, x): (f64, f64)| Ok(libm::atan2(y, x)))?)?;
        math.set("pow", lua.create_function(|_, (x, y): (f64, f64)| Ok(libm::pow(x, y)))?)?;
        // Luau's math.log takes an optional base.
        math.set(
            "log",
            lua.create_function(|_, (x, base): (f64, Option<f64>)| {
                Ok(match base {
                    None => libm::log(x),
                    Some(2.0) => libm::log2(x),
                    Some(10.0) => libm::log10(x),
                    Some(b) => libm::log(x) / libm::log(b),
                })
            })?,
        )?;
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
        // Field layers: temperature, light, whatever mods declare.
        api!("field", (String, i32, i32), |w, (id, x, y)| {
            let f = field_id(w, &id)?;
            w.map.ensure_rooms();
            let defs = w.defs.clone();
            Ok(w.fields.value(&defs, &w.map, f, IVec::new(x, y)))
        });
        api!("ambient", String, |w, id| {
            let f = field_id(w, &id)?;
            Ok(w.fields.ambient(f))
        });
        // Pin a field's outdoor value (tests, tools); nil unpins. Mods that
        // want to change the weather push a named contribution instead.
        api!("set_ambient", (String, Option<f64>), |w, (id, v)| {
            let f = field_id(w, &id)?;
            w.fields.set_ambient(f, v);
            Ok(())
        });
        // A named contribution to a field's outdoor value, easing in over
        // `ease_hours` and expiring after `hours` (nil: until cleared).
        api!("push_ambient", (String, String, f64, Option<f64>, Option<f64>), |w, (id, key, v, hours, ease)| {
            let f = field_id(w, &id)?;
            let tick = w.tick;
            w.fields.push_ambient(f, &key, v, tick, hours, ease.unwrap_or(0.0));
            Ok(())
        });
        api!("clear_ambient", (String, String, Option<f64>), |w, (id, key, ease)| {
            let f = field_id(w, &id)?;
            let tick = w.tick;
            w.fields.clear_ambient(f, &key, tick, ease.unwrap_or(0.0));
            Ok(())
        });
        // Each part of a field's outdoor value: { {label, value}, ... }.
        {
            let ptr = self.world.clone();
            let f = lua.create_function(move |lua, id: String| {
                let parts = with_world(&ptr, |w| {
                    let f = field_id(w, &id)?;
                    Ok(w.fields.explain_ambient(&w.defs, f))
                })?;
                let out = lua.create_table()?;
                for (label, v) in parts {
                    let t = lua.create_table()?;
                    t.set("label", label)?;
                    t.set("value", v)?;
                    out.push(t)?;
                }
                Ok(out)
            })?;
            rim.set("explain", f)?;
        }

        // ---- the calendar
        api!("year", (), |w, _a| Ok(w.year() + 1));
        api!("season", (), |w, _a| Ok(w.season().to_string()));
        {
            let ptr = self.world.clone();
            let f = lua.create_function(move |lua, ()| {
                let t = lua.create_table()?;
                with_world(&ptr, |w| {
                    t.set("year", w.year() + 1)?;
                    t.set("season", w.season())?;
                    t.set("season_index", w.season_index() + 1)?;
                    t.set("day", w.day_of_season())?;
                    t.set("day_of_year", w.day_of_year() + 1)?;
                    t.set("year_days", w.defs.calendar.year_days)?;
                    t.set("year_fraction", crate::terms::from_q(w.clock().year))?;
                    Ok(())
                })?;
                Ok(t)
            })?;
            rim.set("date", f)?;
        }
        let seasons = lua.create_sequence_from(defs.calendar.seasons.iter().map(|s| s.as_str()))?;
        rim.set("seasons", seasons)?;

        // ---- script data and events
        // Plain data kept in the world: hashed, saved, readable by the UI.
        {
            let ptr = self.world.clone();
            let f = lua.create_function(move |_, (key, v): (String, Value)| {
                let d = crate::data::from_lua(&v, &key, 0).map_err(mlua::Error::runtime)?;
                with_world(&ptr, |w| {
                    match d {
                        Some(d) => w.data.insert(key, d),
                        None => w.data.remove(&key),
                    };
                    Ok(())
                })
            })?;
            rim.set("set_data", f)?;
            let ptr = self.world.clone();
            let f = lua.create_function(move |lua, key: String| {
                let d = with_world(&ptr, |w| Ok(w.data.get(&key).cloned()))?;
                match d {
                    Some(d) => crate::data::to_lua(lua, &d),
                    None => Ok(Value::Nil),
                }
            })?;
            rim.set("get_data", f)?;
            // Send an event to `rim.on(name, fn)` handlers in any mod. Mod
            // events are namespaced by the sender: "weather:changed".
            let ptr = self.world.clone();
            let reg = self.reg.clone();
            let f = lua.create_function(move |lua, (name, v): (String, Option<Table>)| {
                let me = calling_mod(lua).unwrap_or_else(|| reg.borrow().current_mod.clone());
                if !name.starts_with(&format!("{me}:")) || name.len() <= me.len() + 1 {
                    return Err(mlua::Error::runtime(format!(
                        "mod '{me}' can only emit its own events, named \"{me}:<event>\" (got \"{name}\")"
                    )));
                }
                let data = match v {
                    Some(t) => crate::data::from_lua(&Value::Table(t), &name, 0).map_err(mlua::Error::runtime)?,
                    None => None,
                };
                with_world(&ptr, |w| {
                    w.events.push(GameEvent::Script { name, data });
                    Ok(())
                })
            })?;
            rim.set("emit", f)?;
        }
        // Sheltered: inside an enclosed room.
        api!("indoors", (i32, i32), |w, (x, y)| {
            w.map.ensure_rooms();
            Ok(w.map.indoors(IVec::new(x, y)))
        });
        // { id, cells, enclosed } for the room at (x, y), or nil on a wall or door.
        {
            let ptr = self.world.clone();
            let f = lua.create_function(move |lua, (x, y): (i32, i32)| {
                let room = with_world(&ptr, |w| {
                    w.map.ensure_rooms();
                    Ok(w.map.room_at(IVec::new(x, y)))
                })?;
                let Some(r) = room else { return Ok(Value::Nil) };
                let t = lua.create_table()?;
                t.set("id", r.id)?;
                t.set("cells", r.cells)?;
                t.set("enclosed", r.enclosed())?;
                Ok(Value::Table(t))
            })?;
            rim.set("room_at", f)?;
        }
        // A thing's stat by name: the def's base times its material's factor.
        // Names the engine never heard of come back as the bare factor, so a
        // mod reads its own numbers off anything built of its material.
        api!("stat", (u64, String), |w, (id, name)| Ok(w.stat(rim_sim_entity(id)?, &name)));
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

        // Mods see `rim` through a proxy. Reads go to the API table; a write
        // may add a new key while mods load, and never replace one, so no mod
        // can swap out the engine's functions or another plugin's API.
        {
            let mut r = self.reg.borrow_mut();
            for pair in rim.pairs::<String, Value>() {
                r.owners.insert(pair?.0, "the engine".to_string());
            }
        }
        lua.set_named_registry_value("rim_api", rim.clone())?;
        let proxy = lua.create_table()?;
        let mt = lua.create_table()?;
        mt.set("__index", rim.clone())?;
        let reg = self.reg.clone();
        let api = rim;
        mt.set(
            "__newindex",
            lua.create_function(move |lua, (_t, k, v): (Table, Value, Value)| {
                let mut r = reg.borrow_mut();
                let me = calling_mod(lua).unwrap_or_else(|| r.current_mod.clone());
                let Value::String(key) = k else {
                    return Err(mlua::Error::runtime(format!("mod '{me}': keys in rim must be strings")));
                };
                let key = key.to_string_lossy().to_string();
                if r.loaded {
                    return Err(mlua::Error::runtime(format!(
                        "mod '{me}' can't set rim.{key}: rim is read-only once mods have loaded (add to it at load time)"
                    )));
                }
                if let Some(owner) = r.owners.get(&key) {
                    return Err(mlua::Error::runtime(format!(
                        "mod '{me}' can't replace rim.{key}: it belongs to {owner}"
                    )));
                }
                api.raw_set(key.as_str(), v)?;
                r.owners.insert(key, format!("mod '{me}'"));
                Ok(())
            })?,
        )?;
        // getmetatable(rim) can't reach the proxy's workings.
        mt.set("__metatable", "rim")?;
        proxy.set_metatable(Some(mt))?;
        g.set("rim", proxy)?;
        Ok(())
    }

    fn call(&self, w: &mut World, prof: &mut Profile, mod_id: &str, f: &Function, args: impl IntoLuaMulti) {
        self.world.0.set(w as *mut World);
        self.steps.set(STEP_BUDGET);
        self.ran_away.set(false);
        self.reg.borrow_mut().current_mod = mod_id.to_string();
        let t = Instant::now();
        let r = f.call::<()>(args);
        self.world.0.set(std::ptr::null_mut());
        prof.add(&format!("mod:{mod_id}"), t.elapsed().as_secs_f64() * 1e6);
        self.reg.borrow_mut().current_mod.clear();
        if let Err(e) = r {
            let text = format!("[{mod_id}] script error: {e}");
            eprintln!("{text}");
            w.message(text, MsgKind::Bad);
            // A runaway won't behave better next time: stop calling it.
            if self.ran_away.get() {
                self.reg.borrow_mut().disabled.insert(f.to_pointer() as usize);
                w.message(
                    format!("[{mod_id}] a hook ran past its step budget and has been switched off for this game."),
                    MsgKind::Bad,
                );
            }
        }
    }

    pub fn run_hooks(&self, w: &mut World, prof: &mut Profile) {
        let due: Vec<(Function, String)> = {
            let r = self.reg.borrow();
            r.hooks
                .iter()
                .filter(|h| (w.tick + h.phase).is_multiple_of(h.interval))
                .filter(|h| !r.disabled.contains(&(h.func.to_pointer() as usize)))
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
                r.handlers
                    .iter()
                    .filter(|h| h.event == name && !r.disabled.contains(&(h.func.to_pointer() as usize)))
                    .map(|h| (h.func.clone(), h.mod_id.clone()))
                    .collect()
            };
            for (f, m) in targets {
                self.call(w, prof, &m, &f, t.clone());
            }
        }
    }

    fn event_table(&self, w: &World, ev: &GameEvent) -> mlua::Result<(String, Table)> {
        if let GameEvent::Script { name, data } = ev {
            let t = match data.as_ref().map(|d| crate::data::to_lua(&self.lua, d)).transpose()? {
                Some(Value::Table(t)) => t,
                _ => self.lua.create_table()?,
            };
            return Ok((name.clone(), t));
        }
        let (name, t) = self.builtin_event_table(w, ev)?;
        Ok((name.to_string(), t))
    }

    fn builtin_event_table(&self, w: &World, ev: &GameEvent) -> mlua::Result<(&'static str, Table)> {
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
            GameEvent::SeasonChanged { season, index, year } => {
                t.set("season", season.as_str())?;
                t.set("index", index + 1)?;
                t.set("year", year + 1)?;
                "season_changed"
            }
            GameEvent::Script { .. } => unreachable!("handled in event_table"),
            GameEvent::ColonyLost => "colony_lost",
        };
        Ok((name, t))
    }

    pub fn hook_count(&self) -> (usize, usize) {
        let r = self.reg.borrow();
        (r.hooks.len(), r.handlers.len())
    }
}
