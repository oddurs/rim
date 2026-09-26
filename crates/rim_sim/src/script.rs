//! Luau plugin host.
//!
//! Scripts run once at load to register hooks (`rim.every`) and event
//! handlers (`rim.on`). The world-facing API (`rim.spawn_pawn`, `rim.wealth`,
//! ...) only works inside those callbacks. It draws from the world's RNG, and
//! `math.random`/`os` are removed, so scripts stay deterministic.
//!
//! Each script gets its own global environment (reads fall through to the
//! shared globals), so mods can't clobber each other by accident. `rim` is
//! the engine's and read-only. Mods share code as modules (DESIGN.md §10):
//! a script returns its exports, and `require("@core/scripts/storyteller")`
//! gets them, but only from a mod listed in `depends` or `optional`.

use crate::modloader::ScriptSource;
use crate::path::Goal;
use crate::profile::Profile;
use crate::world::*;
use crate::{IVec, TICKS_PER_DAY};
use mlua::chunk::Compiler;
use mlua::{Function, IntoLuaMulti, Lua, LuaOptions, StdLib, Table, Value, VmState};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
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

/// A def id a script passed: bare ids are the calling mod's own.
fn def_id(w: &World, kind: &'static str, id: &str, from: &str) -> mlua::Result<crate::defs::DefId> {
    w.defs.resolve(kind, id, from).map_err(mlua::Error::runtime)
}

fn field_id(w: &World, id: &str, from: &str) -> mlua::Result<usize> {
    def_id(w, "field", id, from).map(|f| f as usize)
}

/// The mod whose code is calling into the engine: the chunk name of the
/// nearest Luau frame ("@weather/scripts/weather.luau" is weather's). Not
/// the mod whose hook is running: when mod B calls weather's `force`, it's
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
    /// Set once every script has loaded: `require` only works before.
    loaded: bool,
    /// Hooks and handlers stopped for running away (by function pointer).
    disabled: std::collections::HashSet<usize>,
    hooks: Vec<Hook>,
    handlers: Vec<Handler>,
    /// Each mod's `rim.on_migrate` function.
    migrators: BTreeMap<String, Function>,
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

/// One member of the engine's `rim` table, declared where it's registered:
/// the single source for the Luau type definitions (`types/rim.d.luau`) and
/// the API reference (`docs/modding/api.md`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiDoc {
    pub name: &'static str,
    /// A Luau type: `(x: number) -> number`, or a value's type.
    pub sig: &'static str,
    pub doc: &'static str,
}

/// Types the `rim` declarations refer to.
pub const RIM_TYPES: &str = r#"type Faction = "player" | "hostile" | "wild"
type MessageKind = "info" | "good" | "threat" | "bad"
type CreatureInfo = { id: string, label: string, intelligent: boolean, aggressive: boolean, flees: boolean, plural: string, market_value: number, max_hp: number, wild: boolean }
type ThingInfo = { id: string, label: string, market_value: number, food: boolean, item: boolean, tags: { string } }
type Date = { year: number, season: string, season_index: number, day: number, day_of_year: number, year_days: number, year_fraction: number }
type Room = { id: number, cells: number, enclosed: boolean }
type PriorityPart = { label: string, delta: number }
type WorkWhy = { work: string, level: number, why: string, dist: number? }
type Taker = { id: number, ticks: number }
type Part = { label: string, value: number }
type OrderNeed = { thing: string?, tag: string?, count: number }
type OrderSpec = { label: string, needs: { OrderNeed }, work: number, work_type: string, requires: { string }? }
type OrderInput = { thing: string?, tag: string?, count: number, have: number }
type OrderInfo = { owner: string, label: string, needs: { OrderInput }, work: number, done: number, total: number, requires: { string } }
type ItemQuery = { thing: string?, tag: string? }
type ThingAt = { id: number, thing: string, x: number, y: number, count: number, blueprint: boolean }
"#;

/// A work order's needs, at most: a recipe, not a shopping list.
const MAX_NEEDS: usize = 16;

pub struct ScriptHost {
    /// The engine's `rim` members, as declared at registration.
    api_docs: RefCell<Vec<ApiDoc>>,
    /// Load-time advice for mod authors (determinism hazards).
    pub warnings: Vec<String>,
    lua: Lua,
    world: Rc<WorldPtr>,
    reg: Rc<RefCell<Registry>>,
    /// Interrupts left for the current call (see `STEP_BUDGET`).
    steps: Rc<Cell<u64>>,
    /// The current call used up its step budget.
    ran_away: Rc<Cell<bool>>,
    /// Every script, and what the loaded ones exported.
    modules: Option<Rc<Modules>>,
    /// Print script errors to stderr as well as the message feed. Tools that
    /// report errors themselves (`rim check`) turn it off.
    pub echo_errors: bool,
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

/// Mod scripts as modules (DESIGN.md §10). Keys are paths from the mods
/// folder: "core/scripts/storyteller.luau".
struct Modules {
    /// Every script: its mod and source.
    sources: BTreeMap<String, (String, String)>,
    /// Each installed mod's `depends` and `optional`.
    deps: BTreeMap<String, (Vec<String>, Vec<String>)>,
    reg: Rc<RefCell<Registry>>,
    /// What each script returned, once it has run.
    done: RefCell<BTreeMap<String, Value>>,
    /// Scripts running now, outermost first, to name a cycle.
    stack: RefCell<Vec<String>>,
    /// Mods whose top-level scripts have all run. Their exports are frozen,
    /// so later mods can use them but not change them.
    sealed: RefCell<BTreeSet<String>>,
}

impl Modules {
    fn seal(&self, mod_id: &str) {
        self.sealed.borrow_mut().insert(mod_id.to_string());
        let prefix = format!("{mod_id}/");
        for (_, v) in self.done.borrow().iter().filter(|(k, _)| k.starts_with(&prefix)) {
            freeze(v);
        }
    }
}

/// A module's exports can't be changed once its mod has loaded.
fn freeze(v: &Value) {
    if let Value::Table(t) = v {
        t.set_readonly(true);
    }
}

/// Where `require(path)` in the script `from` points: its mod, and its path
/// without the extension ("core/scripts/storyteller"). `@mod/scripts/x` names
/// any mod's script; `./x` and `../x` are relative to `from`, within its mod.
fn resolve_require(from: &str, path: &str) -> Result<(String, String), String> {
    let joined = if let Some(rest) = path.strip_prefix('@') {
        rest.to_string()
    } else if path.starts_with("./") || path.starts_with("../") {
        let dir = from.rsplit_once('/').map_or("", |(d, _)| d);
        format!("{dir}/{path}")
    } else {
        return Err(format!("require(\"{path}\"): use \"@<mod>/scripts/<name>\", or \"./<name>\" within your own mod"));
    };
    let mut parts: Vec<&str> = Vec::new();
    for c in joined.split('/') {
        match c {
            "" | "." => {}
            ".." if parts.len() > 2 => {
                parts.pop();
            }
            ".." => return Err(format!("require(\"{path}\"): leaves the mod's scripts folder")),
            c => parts.push(c),
        }
    }
    if parts.len() < 3 || parts[1] != "scripts" {
        return Err(format!(
            "require(\"{path}\"): sim modules live in a mod's scripts folder: \"@<mod>/scripts/<name>\""
        ));
    }
    Ok((parts[0].to_string(), parts.join("/")))
}

/// A script's own global environment, with its own `require`. Its globals
/// land here, and reads fall through to the shared globals.
///
/// Marked safe so Luau takes its fast paths (cached imports like
/// `math.floor`, builtin fastcalls, fast `pairs`); without it every global
/// access is a full lookup. It's sound because the globals, `rim` and loaded
/// modules' exports are all read-only.
fn module_env(lua: &Lua, m: &Rc<Modules>, key: &str) -> mlua::Result<Table> {
    let env = lua.create_table()?;
    let (m2, from) = (m.clone(), key.to_string());
    let require = lua.create_function(move |lua, path: String| {
        let (target, base) = resolve_require(&from, &path).map_err(mlua::Error::runtime)?;
        let me = from.split('/').next().unwrap_or_default();
        if target != me {
            let (depends, optional) = m2.deps.get(me).cloned().unwrap_or_default();
            let optional = optional.contains(&target);
            if !optional && !depends.contains(&target) {
                return Err(mlua::Error::runtime(format!(
                    "mod '{me}' requires \"{path}\" but doesn't list '{target}' in depends or optional (mod.toml)"
                )));
            }
            if optional && !m2.deps.contains_key(&target) {
                return Ok(Value::Nil);
            }
        }
        let key = [format!("{base}.luau"), format!("{base}/init.luau")]
            .into_iter()
            .find(|k| m2.sources.contains_key(k))
            .ok_or_else(|| mlua::Error::runtime(format!("require(\"{path}\"): there's no {base}.luau")))?;
        run_module(lua, &m2, &key)
    })?;
    env.raw_set("require", require)?;
    let mt = lua.create_table()?;
    mt.set("__index", lua.globals())?;
    env.set_metatable(Some(mt))?;
    env.set_safeenv(true);
    Ok(env)
}

/// Run a script once and remember what it returned.
fn run_module(lua: &Lua, m: &Rc<Modules>, key: &str) -> mlua::Result<Value> {
    if let Some(v) = m.done.borrow().get(key) {
        return Ok(v.clone());
    }
    let cycle = {
        let stack = m.stack.borrow();
        stack.iter().position(|k| k == key).map(|i| {
            let chain: Vec<&str> = stack[i..].iter().map(String::as_str).chain([key]).collect();
            chain.join(" -> ")
        })
    };
    if let Some(chain) = cycle {
        return Err(mlua::Error::runtime(format!("require cycle: {chain}")));
    }
    if m.reg.borrow().loaded {
        return Err(mlua::Error::runtime(format!(
            "{key} hasn't been loaded: require modules while scripts load, at the top of a script"
        )));
    }
    let (mod_id, source) = &m.sources[key];
    let env = module_env(lua, m, key)?;
    m.stack.borrow_mut().push(key.to_string());
    let prev = std::mem::replace(&mut m.reg.borrow_mut().current_mod, mod_id.clone());
    let result = lua.load(source.as_str()).set_name(format!("@{key}")).set_environment(env).eval::<Value>();
    m.reg.borrow_mut().current_mod = prev;
    m.stack.borrow_mut().pop();
    let v = result?;
    if m.sealed.borrow().contains(mod_id) {
        freeze(&v);
    }
    m.done.borrow_mut().insert(key.to_string(), v.clone());
    Ok(v)
}

impl ScriptHost {
    /// Run every mod's top-level scripts, mods in load order (`mods`), each
    /// mod's scripts by name. Modules they require run first, once.
    pub fn load(
        mods: &[crate::modloader::ModManifest],
        scripts: &[ScriptSource],
        defs: &crate::defs::DefDb,
    ) -> Result<Self, String> {
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
            api_docs: RefCell::new(Vec::new()),
            warnings: Vec::new(),
            lua,
            world: Rc::new(WorldPtr(Cell::new(std::ptr::null_mut()))),
            reg: Rc::new(RefCell::new(Registry::default())),
            steps,
            ran_away,
            modules: None,
            echo_errors: true,
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
        }
        let modules = Rc::new(Modules {
            sources: scripts
                .iter()
                .map(|s| (format!("{}/scripts/{}", s.mod_id, s.name), (s.mod_id.clone(), s.source.clone())))
                .collect(),
            deps: mods.iter().map(|m| (m.id.clone(), (m.depends.clone(), m.optional.clone()))).collect(),
            reg: host.reg.clone(),
            done: RefCell::default(),
            stack: RefCell::default(),
            sealed: RefCell::default(),
        });
        for m in mods {
            for s in scripts.iter().filter(|s| s.entry && s.mod_id == m.id) {
                host.steps.set(STEP_BUDGET);
                let key = format!("{}/scripts/{}", s.mod_id, s.name);
                run_module(&host.lua, &modules, &key).map_err(|e| format!("{key}: {e}"))?;
            }
            modules.seal(&m.id);
        }
        let mut r = host.reg.borrow_mut();
        r.loaded = true;
        r.current_mod.clear();
        drop(r);
        host.modules = Some(modules);
        Ok(host)
    }

    fn declare(&self, name: &'static str, sig: &'static str, doc: &'static str) {
        self.api_docs.borrow_mut().push(ApiDoc { name, sig, doc });
    }

    /// The engine's `rim` members, by name.
    pub fn api(&self) -> Vec<ApiDoc> {
        let mut v = self.api_docs.borrow().clone();
        v.sort_by_key(|d| d.name);
        v
    }

    /// Every name the engine put in `rim` (declared or not: tests compare).
    pub fn engine_names(&self) -> Vec<String> {
        let api: Table = self.lua.named_registry_value("rim_api").expect("rim is installed");
        let mut v: Vec<String> = api.pairs::<String, Value>().filter_map(|p| p.ok().map(|(k, _)| k)).collect();
        v.sort();
        v
    }

    /// Luau type definitions for sim scripts (luau-lsp `definitionFiles`).
    /// Plugins' additions to `rim` are typed `any`.
    pub fn luau_definitions(&self) -> String {
        let mut out = String::from("-- Generated from crates/rim_sim/src/script.rs: don't edit. Regenerate with\n-- RIM_UPDATE_TYPES=1 cargo test -p rim_sim --test api_types\n\n");
        out.push_str(RIM_TYPES);
        out.push_str("\ndeclare rim: {\n");
        for d in self.api() {
            out.push_str(&format!("    -- {}\n    {}: {},\n", d.doc, d.name, d.sig));
        }
        out.push_str("}\n");
        out
    }

    /// The reference for sim scripts' `rim` table, as Markdown.
    pub fn api_reference(&self) -> String {
        let mut out = String::from(
            "# Script API reference: `rim`\n\n\
             Generated from `crates/rim_sim/src/script.rs`, where each function is\n\
             registered: don't edit. Regenerate with\n\
             `RIM_UPDATE_TYPES=1 cargo test -p rim_sim --test api_types`. Types for\n\
             editors are in [`types/rim.d.luau`](../../types/rim.d.luau); the rules\n\
             are in [Scripting rules](scripting.md).\n\n\
             | Name | Type | What it does |\n|---|---|---|\n",
        );
        for d in self.api() {
            let sig = d.sig.replace('|', "\\|");
            out.push_str(&format!("| `rim.{}` | `{sig}` | {} |\n", d.name, d.doc));
        }
        out.push_str("\nTypes used above:\n\n```lua\n");
        out.push_str(RIM_TYPES);
        out.push_str("```\n");
        out
    }

    /// Make the standard libraries and the global table read-only, so no mod
    /// can break another's `math` or `string`.
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
        self.declare("api_version", "string", "The engine's plugin API version, \"MAJOR.MINOR\".");
        rim.set("ticks_per_day", TICKS_PER_DAY)?;
        self.declare("ticks_per_day", "number", "Ticks in a game day.");

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
        self.declare("creature_defs", "{CreatureInfo}", "Every creature def.");
        // Entries of the def kinds mods declare, as fresh tables, so a
        // script may adjust the ones it registers.
        {
            let mod_defs = defs.mod_defs.clone();
            rim.set(
                "defs",
                lua.create_function(move |lua, kind: String| {
                    let from = calling_mod(lua).unwrap_or_default();
                    let full = if kind.contains(':') { kind.clone() } else { format!("{from}:{kind}") };
                    let Some(list) = mod_defs.get(&full) else {
                        let known: Vec<&str> = mod_defs.keys().map(String::as_str).collect();
                        return Err(mlua::Error::runtime(format!(
                            "no def kind '{full}' (declared: {})",
                            if known.is_empty() { "none".to_string() } else { known.join(", ") }
                        )));
                    };
                    let out = lua.create_table()?;
                    for d in list {
                        out.push(crate::data::to_lua(lua, d)?)?;
                    }
                    Ok(out)
                })?,
            )?;
        }
        self.declare(
            "defs",
            "(kind: string) -> { {[string]: any} }",
            "Entries of a def kind a mod declared with [[kind]], in load order: \"type\" for your own kind, \"weather:type\" for another mod's.",
        );
        let things = lua.create_table()?;
        for td in &defs.things {
            let t = lua.create_table()?;
            t.set("id", td.id.as_str())?;
            t.set("label", td.label.as_str())?;
            t.set("market_value", td.market_value)?;
            t.set("food", td.food.is_some())?;
            t.set("item", td.category == crate::defs::Category::Item)?;
            t.set("tags", lua.create_sequence_from(td.tags.iter().map(String::as_str))?)?;
            things.push(t)?;
        }
        rim.set("thing_defs", things)?;
        self.declare("thing_defs", "{ThingInfo}", "Every thing def.");

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
        self.declare(
            "every",
            "(interval: number, fn: () -> ()) -> ()",
            "Run fn every `interval` ticks (hooks are staggered). Register at load time.",
        );
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
        self.declare(
            "on",
            "(event: string, fn: (event: {[string]: any}) -> ()) -> ()",
            "Handle an engine event (`pawn_died`, `season_changed`, ...) or a mod event (`weather:changed`).",
        );
        let reg = self.reg.clone();
        rim.set(
            "on_migrate",
            lua.create_function(move |_, func: Function| {
                let mut r = reg.borrow_mut();
                let mod_id = r.current_mod.clone();
                if r.loaded {
                    return Err(mlua::Error::runtime(format!("mod '{mod_id}': register on_migrate at load time")));
                }
                if r.migrators.contains_key(&mod_id) {
                    return Err(mlua::Error::runtime(format!("mod '{mod_id}' registered on_migrate twice")));
                }
                r.migrators.insert(mod_id, func);
                Ok(())
            })?,
        )?;
        self.declare(
            "on_migrate",
            "(fn: (from_version: string, data: {[string]: any}) -> {[string]: any}) -> ()",
            "Upgrade your script data from a save made with a different version of your mod: fn gets that version and \
             your data (bare keys) and returns the data to keep. It sees no world: only your data. Runs on load, \
             before any hook. Register at load time.",
        );
        let reg = self.reg.clone();
        rim.set(
            "log",
            lua.create_function(move |_, msg: String| {
                eprintln!("[{}] {msg}", reg.borrow().current_mod);
                Ok(())
            })?,
        )?;
        self.declare("log", "(message: string) -> ()", "Print a line to the console, tagged with your mod.");

        // ---- world API (only valid inside callbacks)
        macro_rules! api {
            ($name:literal, $sig:literal, $doc:literal, $ty:ty, |$w:ident, $args:pat_param| $body:expr) => {{
                let ptr = self.world.clone();
                rim.set($name, lua.create_function(move |_, $args: $ty| with_world(&ptr, |$w| $body))?)?;
                self.declare($name, $sig, $doc);
            }};
            // `$from` is the mod whose code is calling: bare def ids are its own.
            ($name:literal, $sig:literal, $doc:literal, $ty:ty, |$w:ident, $from:ident, $args:pat_param| $body:expr) => {{
                let ptr = self.world.clone();
                rim.set(
                    $name,
                    lua.create_function(move |lua, $args: $ty| {
                        let $from = calling_mod(lua).unwrap_or_default();
                        with_world(&ptr, |$w| $body)
                    })?,
                )?;
                self.declare($name, $sig, $doc);
            }};
        }

        api!("tick", "() -> number", "The current tick. A day is `rim.ticks_per_day` ticks.", (), |w, _a| Ok(w.tick));
        api!("day", "() -> number", "Days since the game began, from 0.", (), |w, _a| Ok(w.day()));
        api!("hour", "() -> number", "Hour of the day, 0 to 24 (tick 0 is 06:00).", (), |w, _a| Ok(w.hour()));
        api!("wealth", "() -> number", "The colony's wealth (recomputed every few hundred ticks).", (), |w, _a| Ok(
            w.wealth
        ));
        api!("map_size", "() -> (number, number)", "Map width and height in cells.", (), |w, _a| Ok((
            w.map.w, w.map.h
        )));
        api!(
            "random",
            "() -> number",
            "A number in [0, 1) from the world's random numbers: the same on every machine.",
            (),
            |w, _a| Ok(w.rng.float())
        );
        api!(
            "random_int",
            "(lo: number, hi: number) -> number",
            "A whole number from lo to hi inclusive, from the world's random numbers.",
            (i32, i32),
            |w, (a, b)| Ok(w.rng.range(a, b))
        );
        api!("colonists", "() -> number", "How many colonists are alive.", (), |w, _a| Ok(w.colonists().count()));
        api!(
            "colony_center",
            "() -> (number?, number?)",
            "The colonists' average cell, or nil if there are none.",
            (),
            |w, _a| Ok(match w.colony_center() {
                Some(c) => (Some(c.x), Some(c.y)),
                None => (None, None),
            })
        );
        api!("count_pawns", "(faction: Faction) -> number", "Living pawns of a faction.", String, |w, faction| {
            let f =
                Faction::parse(&faction).ok_or_else(|| mlua::Error::runtime(format!("unknown faction '{faction}'")))?;
            Ok(w.pawns
                .iter()
                .filter(|&&e| w.ecs.get::<&Pawn>(e).is_ok_and(|p| p.active && !p.dead && p.faction == f))
                .count())
        });
        // Rough melee output of the colony: what raids are weighed against.
        api!(
            "colony_strength",
            "() -> number",
            "Rough melee output of the colony, which raids are weighed against.",
            (),
            |w, _a| {
                let defs = w.defs.clone();
                let mut s = 0.0;
                for e in w.colonists() {
                    if let Ok(p) = w.ecs.get::<&Pawn>(e) {
                        let cd = defs.creature(p.def);
                        s +=
                            (p.hp as f64 / cd.max_hp as f64) * cd.melee_damage as f64 * 60.0 / cd.melee_cooldown as f64;
                    }
                }
                Ok(s)
            }
        );
        api!(
            "message",
            "(text: string, kind: MessageKind?) -> ()",
            "Post a message to the feed (default kind \"info\").",
            (String, Option<String>),
            |w, (text, kind)| {
                w.message(text, MsgKind::parse(kind.as_deref().unwrap_or("info")));
                Ok(())
            }
        );
        // A random open cell on the map edge that can reach the colony.
        api!(
            "edge_cell",
            "() -> (number?, number?)",
            "A random open cell on the map edge that can reach the colony.",
            (),
            |w, _a| {
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
            }
        );
        // A random open cell within `r` of (x, y).
        api!(
            "near_cell",
            "(x: number, y: number, r: number) -> (number?, number?)",
            "A random open cell within r of (x, y).",
            (i32, i32, i32),
            |w, (x, y, r)| {
                for _ in 0..100 {
                    let p = IVec::new(x + w.rng.range(-r, r), y + w.rng.range(-r, r));
                    if w.map.passable(p) {
                        return Ok((Some(p.x), Some(p.y)));
                    }
                }
                Ok((None, None))
            }
        );
        api!(
            "spawn_pawn",
            "(creature: string, faction: Faction, x: number, y: number, name: string?) -> (number?, string?)",
            "Spawn a creature; returns its id and name, or nil if the cell is blocked.",
            (String, String, i32, i32, Option<String>),
            |w, from, (creature, faction, x, y, name)| {
                let def = def_id(w, "creature", &creature, &from)?;
                let f = Faction::parse(&faction)
                    .ok_or_else(|| mlua::Error::runtime(format!("unknown faction '{faction}'")))?;
                let p = IVec::new(x, y);
                if !w.map.passable(p) {
                    return Ok((None, None));
                }
                let e = w.spawn_pawn(def, f, p, name);
                let name = w.ecs.get::<&Pawn>(e).map(|p| p.name.clone()).unwrap_or_default();
                Ok((Some(e.to_bits().get()), Some(name)))
            }
        );
        // Field layers: temperature, light, whatever mods declare.
        api!(
            "field",
            "(id: string, x: number, y: number) -> number",
            "A field's value at a cell (temperature, light, ...).",
            (String, i32, i32),
            |w, from, (id, x, y)| {
                let f = field_id(w, &id, &from)?;
                w.map.ensure_rooms();
                let defs = w.defs.clone();
                Ok(w.fields.value(&defs, &w.map, f, IVec::new(x, y)))
            }
        );
        api!("ambient", "(id: string) -> number", "A field's outdoor value.", String, |w, from, id| {
            let f = field_id(w, &id, &from)?;
            Ok(w.fields.ambient(f))
        });
        // Pin a field's outdoor value (tests, tools); nil unpins. Mods that
        // want to change the weather push a named contribution instead.
        api!("set_ambient", "(id: string, value: number?) -> ()", "Pin a field's outdoor value, overriding its terms and pushes; nil unpins. For tests and tools: mods push instead.", (String, Option<f64>), |w, from, (id, v)| {
            let f = field_id(w, &id, &from)?;
            w.fields.set_ambient(f, v);
            Ok(())
        });
        // A named contribution to a field's outdoor value, easing in over
        // `ease_hours` and expiring after `hours` (nil: until cleared).
        api!("push_ambient", "(field: string, key: string, value: number, hours: number?, ease_hours: number?) -> ()", "Add a named contribution to a field's outdoor value, easing in over ease_hours and expiring after hours (nil: until cleared).", (String, String, f64, Option<f64>, Option<f64>), |w, from, (id, key, v, hours, ease)| {
            let f = field_id(w, &id, &from)?;
            let tick = w.tick;
            w.fields.push_ambient(f, &key, v, tick, hours, ease.unwrap_or(0.0));
            Ok(())
        });
        api!(
            "clear_ambient",
            "(field: string, key: string, ease_hours: number?) -> ()",
            "Ease a named contribution out and remove it.",
            (String, String, Option<f64>),
            |w, from, (id, key, ease)| {
                let f = field_id(w, &id, &from)?;
                let tick = w.tick;
                w.fields.clear_ambient(f, &key, tick, ease.unwrap_or(0.0));
                Ok(())
            }
        );
        // Each part of a field's outdoor value: { {label, value}, ... }.
        {
            let ptr = self.world.clone();
            let f = lua.create_function(move |lua, id: String| {
                let from = calling_mod(lua).unwrap_or_default();
                let parts = with_world(&ptr, |w| {
                    let f = field_id(w, &id, &from)?;
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
            self.declare(
                "explain",
                "(field: string) -> {Part}",
                "Each part of a field's outdoor value: its terms, then pushes.",
            );
        }

        // ---- the calendar
        api!("year", "() -> number", "The year, from 1.", (), |w, _a| Ok(w.year() + 1));
        api!("season", "() -> string", "The current season's name.", (), |w, _a| Ok(w.season().to_string()));
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
            self.declare("date", "() -> Date", "The calendar date.");
        }
        let seasons = lua.create_sequence_from(defs.calendar.seasons.iter().map(|s| s.as_str()))?;
        rim.set("seasons", seasons)?;
        self.declare("seasons", "{string}", "The calendar's season names, in order.");

        // ---- script data and events
        // Plain data kept in the world: hashed, saved, readable by the UI.
        // Keys belong to a mod: a bare key is the caller's own, and a mod
        // writes only under its own name, so a mod's saved state is exactly
        // its keys (DESIGN.md §7a).
        {
            let ptr = self.world.clone();
            let reg = self.reg.clone();
            let f = lua.create_function(move |lua, (key, v): (String, Value)| {
                let me = calling_mod(lua).unwrap_or_else(|| reg.borrow().current_mod.clone());
                let key = match key.split_once(':') {
                    None => format!("{me}:{key}"),
                    Some((owner, _)) if owner == me => key,
                    Some((owner, _)) => {
                        return Err(mlua::Error::runtime(format!(
                            "mod '{me}' can't write \"{key}\": script data under \"{owner}:\" is {owner}'s. \
                             Write \"{me}:<key>\", or a bare key"
                        )))
                    }
                };
                if key.len() <= me.len() + 1 {
                    return Err(mlua::Error::runtime(format!("mod '{me}': a script data key can't be empty")));
                }
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
            self.declare(
                "set_data",
                "(key: string, value: any) -> ()",
                "Keep plain data in the world (hashed, saved, readable by the UI as view.data). A bare key is your \
                 mod's (\"state\" is \"your_mod:state\"); you can't write another mod's.",
            );
            let ptr = self.world.clone();
            let reg = self.reg.clone();
            let f = lua.create_function(move |lua, key: String| {
                let key = if key.contains(':') {
                    key
                } else {
                    let me = calling_mod(lua).unwrap_or_else(|| reg.borrow().current_mod.clone());
                    format!("{me}:{key}")
                };
                let d = with_world(&ptr, |w| Ok(w.data.get(&key).cloned()))?;
                match d {
                    Some(d) => crate::data::to_lua(lua, &d),
                    None => Ok(Value::Nil),
                }
            })?;
            rim.set("get_data", f)?;
            self.declare(
                "get_data",
                "(key: string) -> any",
                "A copy of stored script data, or nil. A bare key is your mod's; \"weather:forecast\" reads another's.",
            );
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
            self.declare(
                "emit",
                "(name: string, data: {[string]: any}?) -> ()",
                "Send an event to rim.on handlers in any mod. Only under your own name: \"your_mod:event\".",
            );
        }
        // Sheltered: inside an enclosed room.
        api!(
            "indoors",
            "(x: number, y: number) -> boolean",
            "Whether a cell is inside an enclosed room.",
            (i32, i32),
            |w, (x, y)| {
                w.map.ensure_rooms();
                Ok(w.map.indoors(IVec::new(x, y)))
            }
        );
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
            self.declare("room_at", "(x: number, y: number) -> Room?", "The room at a cell, or nil on a wall or door.");
        }
        // A thing's stat by name: the def's base times its material's factor.
        // Names the engine never heard of come back as the bare factor, so a
        // mod reads its own numbers off anything built of its material.
        api!(
            "stat",
            "(id: number, name: string) -> number?",
            "A thing's stat by name: its def's base times its material's factor.",
            (u64, String),
            |w, (id, name)| Ok(w.stat(rim_sim_entity(id)?, &name))
        );
        // A colonist's work priority once the rules have had their say.
        api!(
            "priority",
            "(id: number, work: string) -> number?",
            "A colonist's priority for a work type, rules and stance included: 1 first, 0 never. Nil if it isn't a pawn.",
            (u64, String),
            |w, from, (id, work)| {
                let t = def_id(w, "work_type", &work, &from)?;
                let e = rim_sim_entity(id)?;
                Ok(w.ecs.get::<&Pawn>(e).ok().map(|p| crate::rules::effective(&w.defs, &w.rules, &p, t)))
            }
        );
        // Why a priority is what it is: the base, then each rule that moved it.
        {
            let ptr = self.world.clone();
            let f = lua.create_function(move |lua, (id, work): (u64, String)| {
                let from = calling_mod(lua).unwrap_or_default();
                let parts = with_world(&ptr, |w| {
                    let t = def_id(w, "work_type", &work, &from)?;
                    let e = rim_sim_entity(id)?;
                    Ok(w.ecs.get::<&Pawn>(e).ok().map(|p| crate::rules::explain(&w.defs, &w.rules, &p, t).1))
                })?;
                let Some(parts) = parts else { return Ok(Value::Nil) };
                let t = lua.create_table()?;
                for p in parts {
                    let row = lua.create_table()?;
                    row.set("label", p.label)?;
                    row.set("delta", p.delta)?;
                    t.push(row)?;
                }
                Ok(Value::Table(t))
            })?;
            rim.set("priority_parts", f)?;
            // The why panel's data: each work type, and why it's taken or not.
            let ptr = self.world.clone();
            let f = lua.create_function(move |lua, id: u64| {
                let rows = with_world(&ptr, |w| {
                    let e = rim_sim_entity(id)?;
                    Ok(crate::ai::explain_work(w, e)
                        .into_iter()
                        .map(|x| {
                            let why = crate::order::why_text(w, &x.why);
                            (w.defs.work_types[x.work as usize].id.clone(), x.level, why, x.dist)
                        })
                        .collect::<Vec<_>>())
                })?;
                let t = lua.create_table()?;
                for (work, level, why, dist) in rows {
                    let row = lua.create_table()?;
                    row.set("work", work)?;
                    row.set("level", level)?;
                    row.set("why", why)?;
                    row.set("dist", dist)?;
                    t.push(row)?;
                }
                Ok(t)
            })?;
            rim.set("explain_work", f)?;
            self.declare(
                "explain_work",
                "(id: number) -> { WorkWhy }",
                "Why a colonist would do what it would, and passes over the rest, work type by work type in tie-break order. Empty if it isn't a pawn.",
            );
            let ptr = self.world.clone();
            let f = lua.create_function(move |lua, id: u64| {
                let line = with_world(&ptr, |w| Ok(crate::ai::who_takes(w, rim_sim_entity(id)?)))?;
                let t = lua.create_table()?;
                for (e, ticks) in line {
                    let row = lua.create_table()?;
                    row.set("id", e.to_bits().get())?;
                    row.set("ticks", ticks)?;
                    t.push(row)?;
                }
                Ok(t)
            })?;
            rim.set("who_takes", f)?;
            self.declare(
                "who_takes",
                "(id: number) -> { Taker }",
                "Who would take the job on a thing next, soonest first, with about how many ticks until they're there: colonists free to choose. Empty if someone already holds it.",
            );
            self.declare(
                "priority_parts",
                "(id: number, work: string) -> { PriorityPart }?",
                "How a colonist's priority came about: the base, then each rule that moved it. The deltas sum to rim.priority.",
            );
        }
        api!("stance", "() -> string?", "The colony's stance, or nil if no mod defines any.", (), |w, _a| Ok(w
            .stance
            .map(|s| w.defs.stances[s as usize].id.clone())));
        // Scripts run inside the tick, so this is as deterministic as a Command.
        api!(
            "set_stance",
            "(stance: string) -> ()",
            "Put the colony in a stance: its priority rules hold until another. For incidents; the player's comes as a command.",
            String,
            |w, from, stance| {
                w.stance = Some(def_id(w, "stance", &stance, &from)?);
                w.update_rules();
                Ok(())
            }
        );
        // Make a pawn give up and walk off the map after `ticks`.
        api!(
            "leave_after",
            "(id: number, ticks: number) -> ()",
            "Make a pawn give up and walk off the map after `ticks`.",
            (u64, u64),
            |w, (id, ticks)| {
                let e = rim_sim_entity(id)?;
                let t = w.tick + ticks;
                if let Ok(mut p) = w.ecs.get::<&mut Pawn>(e) {
                    p.leave_at = Some(t);
                }
                Ok(())
            }
        );
        api!(
            "spawn_item",
            "(thing: string, x: number, y: number, count: number, stuff: string?) -> number",
            "Drop items near a cell, merging into stacks; returns how many didn't fit. stuff is what they're made \
             of (a flint axe): it sets their hp and quality, and they stack only with the same.",
            (String, i32, i32, u32, Option<String>),
            |w, from, (thing, x, y, count, stuff)| {
                let def = def_id(w, "thing", &thing, &from)?;
                let stuff = match stuff {
                    None => None,
                    Some(s) => {
                        let m = def_id(w, "thing", &s, &from)?;
                        if w.defs.thing(m).stuff.is_none() {
                            return Err(mlua::Error::runtime(format!("spawn_item: {s} isn't a material")));
                        }
                        Some(m)
                    }
                };
                Ok(w.place_item_of(def, IVec::new(x, y), count, stuff))
            }
        );
        // Whether a job needing these tool tags could be worked now: some
        // tool in the colony, lying about or in hand, covers every one.
        api!(
            "has_tool",
            "(tags: { string }) -> boolean",
            "Whether some tool in the colony, lying about or in a hand, has every one of these tool tags. False for a tag no tool has.",
            Vec<String>,
            |w, tags| Ok(w.defs.tool_mask(&tags).is_some_and(|need| w.colony_tools() & need == need))
        );
        api!(
            "count_items",
            "(what: ItemQuery) -> number",
            "Items lying on the map, by thing ({ thing = \"core:wood\" }) or by tag ({ tag = \"knappable\" }).",
            Table,
            |w, from, q| {
                let thing: Option<String> = q.get("thing")?;
                let tag: Option<String> = q.get("tag")?;
                let def = thing.map(|t| def_id(w, "thing", &t, &from)).transpose()?;
                let takes = |d: crate::defs::DefId| match (&def, &tag) {
                    (Some(x), _) => *x == d,
                    (None, Some(tag)) => w.defs.thing(d).tags.contains(tag),
                    (None, None) => false,
                };
                let n: u32 = w
                    .ecs
                    .query::<(hecs::Entity, &Thing)>()
                    .iter()
                    .filter(|(e, t)| takes(t.def) && w.map.item_at(t.pos) == Some(*e))
                    .map(|(_, t)| t.count)
                    .sum();
                Ok(n)
            }
        );
        api!(
            "post_order",
            "(site: number, order: OrderSpec) -> ()",
            "Post a work order on a thing (a station): bring what `needs` lists, by thing or by tag, then work \
             `work` ticks there, holding a tool with every tag in `requires`. Colonists take it as `work_type` work. \
             When it's done, `order_done` names what went in; make what it makes then. One order a site at a time.",
            (u64, Table),
            |w, from, (site, spec)| {
                let site = rim_sim_entity(site)?;
                let t = w.thing(site).ok_or_else(|| mlua::Error::runtime("post_order: no thing with that id"))?;
                // A site is something built that stands: not an item that
                // could be eaten or carried off, nor a tree to be felled.
                if w.map.fixture_at(t.pos) != Some(site) || w.defs.thing(t.def).natural {
                    return Err(mlua::Error::runtime(
                        "post_order: a site is a building or station, not an item or a plant",
                    ));
                }
                if w.ecs.get::<&Blueprint>(site).is_ok() {
                    return Err(mlua::Error::runtime("post_order: the site is still a plan"));
                }
                if w.ecs.get::<&Order>(site).is_ok() {
                    return Err(mlua::Error::runtime(format!(
                        "post_order: the {} already has an order",
                        w.defs.thing(t.def).label
                    )));
                }
                let label: String = spec.get("label")?;
                let work: u32 = spec.get("work")?;
                let work_type: String = spec.get("work_type")?;
                let work_type = def_id(w, "work_type", &work_type, &from)?;
                let requires: Vec<String> = spec.get::<Option<Vec<String>>>("requires")?.unwrap_or_default();
                if let Some(t) = requires.iter().find(|t| !w.defs.tool_tags.contains(t)) {
                    return Err(mlua::Error::runtime(format!("post_order: no tool has the tag \"{t}\"")));
                }
                let mut needs = Vec::new();
                for n in spec.get::<Table>("needs")?.sequence_values::<Table>() {
                    let n = n?;
                    let thing: Option<String> = n.get("thing")?;
                    let tag: Option<String> = n.get("tag")?;
                    let count: u32 = n.get("count")?;
                    if thing.is_some() == tag.is_some() || count == 0 {
                        return Err(mlua::Error::runtime(
                            "post_order: each need is { thing = ..., count = n } or { tag = ..., count = n }",
                        ));
                    }
                    let thing = thing.map(|t| def_id(w, "thing", &t, &from)).transpose()?;
                    needs.push(Need { thing, tag, count, delivered: Vec::new() });
                }
                if needs.len() > MAX_NEEDS {
                    return Err(mlua::Error::runtime(format!("post_order: at most {MAX_NEEDS} needs")));
                }
                let order =
                    Order { owner: from, label, needs, work: work.max(1), requires, work_type, done: 0, total: 0 };
                let _ = w.ecs.insert_one(site, order);
                w.map.touch(t.pos);
                Ok(())
            }
        );
        api!(
            "cancel_order",
            "(site: number) -> boolean",
            "Take your work order off its site. What was brought is put back down there. False if it had none.",
            u64,
            |w, from, site| {
                let site = rim_sim_entity(site)?;
                match w.ecs.get::<&Order>(site).map(|o| o.owner.clone()) {
                    Err(_) => return Ok(false),
                    Ok(owner) if owner != from => {
                        return Err(mlua::Error::runtime(format!("cancel_order: the order is {owner}'s")))
                    }
                    Ok(_) => {}
                }
                let Ok(o) = w.ecs.remove_one::<Order>(site) else { return Ok(false) };
                if let Some(t) = w.thing(site) {
                    for &lot in o.needs.iter().flat_map(|n| &n.delivered) {
                        w.place_lot(lot, t.pos);
                    }
                    w.map.touch(t.pos);
                }
                Ok(true)
            }
        );
        // A thing by id: what it is and where, or nil if it's gone.
        {
            let ptr = self.world.clone();
            let f = lua.create_function(move |lua, id: u64| {
                let e = rim_sim_entity(id)?;
                with_world(&ptr, |w| {
                    let Some(t) = w.thing(e) else { return Ok(Value::Nil) };
                    let r = lua.create_table()?;
                    r.set("id", id)?;
                    r.set("thing", w.defs.thing(t.def).id.as_str())?;
                    r.set("x", t.pos.x)?;
                    r.set("y", t.pos.y)?;
                    r.set("count", t.count)?;
                    r.set("blueprint", w.ecs.get::<&Blueprint>(e).is_ok())?;
                    Ok(Value::Table(r))
                })
            })?;
            rim.set("thing", f)?;
            self.declare(
                "thing",
                "(id: number) -> ThingAt?",
                "A thing by id: what it is and where, or nil if it's gone.",
            );
        }
        // The order on a site, and how far it's got, or nil.
        {
            let ptr = self.world.clone();
            let f = lua.create_function(move |lua, site: u64| {
                let site = rim_sim_entity(site)?;
                with_world(&ptr, |w| {
                    let Ok(o) = w.ecs.get::<&Order>(site) else { return Ok(Value::Nil) };
                    let t = lua.create_table()?;
                    t.set("owner", o.owner.as_str())?;
                    t.set("label", o.label.as_str())?;
                    t.set("work", o.work)?;
                    t.set("done", o.done)?;
                    t.set("total", if o.total > 0 { o.total } else { o.work })?;
                    t.set("requires", lua.create_sequence_from(o.requires.iter().map(String::as_str))?)?;
                    let needs = lua.create_table()?;
                    for n in &o.needs {
                        let row = lua.create_table()?;
                        row.set("thing", n.thing.map(|d| w.defs.thing(d).id.clone()))?;
                        row.set("tag", n.tag.as_deref())?;
                        row.set("count", n.count)?;
                        row.set("have", n.have())?;
                        needs.push(row)?;
                    }
                    t.set("needs", needs)?;
                    Ok(Value::Table(t))
                })
            })?;
            rim.set("order", f)?;
            self.declare(
                "order",
                "(site: number) -> OrderInfo?",
                "The work order on a thing and how far it's got, or nil.",
            );
        }

        // Mods see `rim` through a proxy: reads go to the API table, and a
        // write is an error that says how to share code instead.
        lua.set_named_registry_value("rim_api", rim.clone())?;
        for pair in rim.pairs::<Value, Value>() {
            if let (_, Value::Table(t)) = pair? {
                t.set_readonly(true);
            }
        }
        rim.set_readonly(true);
        let proxy = lua.create_table()?;
        let mt = lua.create_table()?;
        mt.set("__index", rim)?;
        let reg = self.reg.clone();
        mt.set(
            "__newindex",
            lua.create_function(move |lua, (_t, k, _v): (Table, Value, Value)| -> mlua::Result<()> {
                let me = calling_mod(lua).unwrap_or_else(|| reg.borrow().current_mod.clone());
                let key = k.to_string().unwrap_or_else(|_| "?".into());
                Err(mlua::Error::runtime(format!(
                    "mod '{me}' can't set rim.{key}: rim is the engine's and read-only. To share code, return it \
                     from a script and require it (docs/modding/scripting.md)"
                )))
            })?,
        )?;
        // getmetatable(rim) can't reach the proxy's workings.
        mt.set("__metatable", "rim")?;
        proxy.set_metatable(Some(mt))?;
        g.set("rim", proxy)?;
        Ok(())
    }

    /// Call a function a loaded module exports, like a hook would: for tests
    /// and tools (`rim test`'s `w:call`). `path` is a require path,
    /// "@core/scripts/storyteller"; arguments and the result are plain data.
    pub fn call_export(
        &self,
        w: &mut World,
        path: &str,
        func: &str,
        args: &[Option<crate::data::Data>],
    ) -> Result<Option<crate::data::Data>, String> {
        let modules = self.modules.as_ref().ok_or("scripts haven't loaded")?;
        let (mod_id, base) = resolve_require("", path)?;
        let exports = [format!("{base}.luau"), format!("{base}/init.luau")]
            .iter()
            .find_map(|k| modules.done.borrow().get(k).cloned())
            .ok_or_else(|| format!("{path} isn't loaded (is '{mod_id}' enabled, and does something require it?)"))?;
        let Value::Table(exports) = exports else { return Err(format!("{path} doesn't export a table")) };
        let f: Function = exports.get(func).map_err(|_| format!("{path} doesn't export a function '{func}'"))?;
        let args: Vec<Value> = args
            .iter()
            .map(|a| match a {
                Some(d) => crate::data::to_lua(&self.lua, d),
                None => Ok(Value::Nil),
            })
            .collect::<mlua::Result<_>>()
            .map_err(|e| e.to_string())?;
        self.world.0.set(w as *mut World);
        self.steps.set(STEP_BUDGET);
        self.reg.borrow_mut().current_mod = mod_id;
        let r = f.call::<Value>(mlua::MultiValue::from_iter(args));
        self.world.0.set(std::ptr::null_mut());
        self.reg.borrow_mut().current_mod.clear();
        let v = r.map_err(|e| format!("{path}: {func}: {e}"))?;
        crate::data::from_lua(&v, func, 0)
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
            if self.echo_errors {
                eprintln!("{text}");
            }
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

    /// Run `mod_id`'s migrator, if it has one, over its script data in `w`:
    /// the save was made with version `from`. Only that mod's keys go in,
    /// and what comes back replaces them. The migrator gets no world: a
    /// migration is a function of the mod's data, nothing else.
    pub fn migrate(&self, w: &mut World, mod_id: &str, from: &str) -> Result<(), String> {
        let Some(f) = self.reg.borrow().migrators.get(mod_id).cloned() else { return Ok(()) };
        let prefix = format!("{mod_id}:");
        let mine: BTreeMap<crate::data::Key, crate::data::Data> = w
            .data
            .iter()
            .filter_map(|(k, v)| Some((crate::data::Key::Str(k.strip_prefix(&prefix)?.to_string()), v.clone())))
            .collect();
        let data = crate::data::to_lua(&self.lua, &crate::data::Data::Table(mine)).map_err(|e| e.to_string())?;
        self.steps.set(STEP_BUDGET);
        self.reg.borrow_mut().current_mod = mod_id.to_string();
        let r = f.call::<Value>((from, data));
        self.reg.borrow_mut().current_mod.clear();
        let out = r.map_err(|e| e.to_string())?;
        let Some(crate::data::Data::Table(out)) = crate::data::from_lua(&out, "migrate", 0)? else {
            return Err("on_migrate must return a table of your data".into());
        };
        let mut keep = Vec::new();
        for (k, v) in out {
            match k {
                crate::data::Key::Str(k) if !k.is_empty() => keep.push((format!("{prefix}{k}"), v)),
                _ => return Err("on_migrate must return your data by non-empty string keys".into()),
            }
        }
        w.data.retain(|k, _| !k.starts_with(&prefix));
        w.data.extend(keep);
        Ok(())
    }

    /// Hooks and handlers switched off for running away, by registration
    /// index: function pointers don't survive a load, the order does.
    pub fn disabled(&self) -> (Vec<usize>, Vec<usize>) {
        let r = self.reg.borrow();
        let off = |f: &Function| r.disabled.contains(&(f.to_pointer() as usize));
        (
            r.hooks.iter().enumerate().filter(|(_, h)| off(&h.func)).map(|(i, _)| i).collect(),
            r.handlers.iter().enumerate().filter(|(_, h)| off(&h.func)).map(|(i, _)| i).collect(),
        )
    }

    /// Switch off what `disabled` reported, after a load.
    pub fn set_disabled(&self, hooks: &[usize], handlers: &[usize]) {
        let mut r = self.reg.borrow_mut();
        let fs: Vec<usize> = hooks
            .iter()
            .filter_map(|&i| r.hooks.get(i).map(|h| h.func.to_pointer() as usize))
            .chain(handlers.iter().filter_map(|&i| r.handlers.get(i).map(|h| h.func.to_pointer() as usize)))
            .collect();
        r.disabled.extend(fs);
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
            GameEvent::PawnDied { id, name, def, faction, pos, founder } => {
                t.set("id", id.to_bits().get())?;
                t.set("name", name.as_str())?;
                t.set("creature", defs.creature(*def).id.as_str())?;
                t.set("faction", faction.name())?;
                t.set("x", pos.x)?;
                t.set("y", pos.y)?;
                t.set("founder", *founder)?;
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
            GameEvent::OrderDone { site, owner, label, inputs, stuff } => {
                t.set("site", site.to_bits().get())?;
                if let Some(at) = w.thing(*site).map(|t| t.pos) {
                    t.set("x", at.x)?;
                    t.set("y", at.y)?;
                }
                t.set("owner", owner.as_str())?;
                t.set("label", label.as_str())?;
                let ins = self.lua.create_table()?;
                for lot in inputs {
                    let row = self.lua.create_table()?;
                    row.set("thing", defs.thing(lot.def).id.as_str())?;
                    row.set("count", lot.count)?;
                    row.set("made_of", lot.made_of.map(|m| defs.thing(m).id.clone()))?;
                    ins.push(row)?;
                }
                t.set("inputs", ins)?;
                t.set("stuff", stuff.map(|d| defs.thing(d).id.clone()))?;
                "order_done"
            }
            GameEvent::OrderLost { site, owner, label } => {
                t.set("site", site.to_bits().get())?;
                t.set("owner", owner.as_str())?;
                t.set("label", label.as_str())?;
                "order_lost"
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
