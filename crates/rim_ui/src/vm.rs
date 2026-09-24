//! The client-only UI VM (DESIGN.md §11).
//!
//! UI scripts live in `mods/<id>/ui/*.luau`. They run in their own Luau VM
//! in sandbox mode, separate from the simulation's: `ui`, `view` and `act`
//! are frozen, there is no I/O, and nothing here can reach the simulation
//! except by queueing a `UiAction`.
//!
//! A component is `ui.define(id, function(view) return tree end)`. Mods
//! change each other's UI with four operations applied wherever a node has
//! that id: `ui.extend`, `ui.replace`, `ui.wrap`, `ui.remove`.

use crate::node::{error_node, key_for, node_from_table, Ctx, Node};
use crate::theme::Theme;
use crate::view::{ClientView, UiAction};
use mlua::{Function, Lua, Table, Value};
use rim_sim::defs::Satisfier;
use rim_sim::hecs::Entity;
use rim_sim::world::{Blueprint, Faction, MsgKind, Pawn, Regrow, Thing, World, NEED_MAX};
use rim_sim::TICKS_PER_DAY;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

/// Extra read-only facts the UI engine itself provides (devtools, stats).
#[derive(Clone, Debug, Default)]
pub struct EngineInfo {
    pub font: String,
    pub build_us: f64,
    pub layout_us: f64,
    pub paint_us: f64,
    pub nodes: usize,
    pub layouts: u64,
    /// Devtools: the node under the cursor.
    pub inspect: Option<InspectInfo>,
    /// Devtools: (depth, kind, id, owner) for every node.
    pub tree: Vec<(usize, String, String, String)>,
    pub outlines: bool,
}

#[derive(Clone, Debug, Default)]
pub struct InspectInfo {
    pub id: String,
    pub owner: String,
    pub kind: String,
    pub rect: [f32; 4],
    pub path: String,
}

/// The world and client state lent to scripts while they run.
struct Lent {
    world: Cell<*const World>,
    client: Cell<*const ClientView>,
    engine: Cell<*const EngineInfo>,
}

struct Lease<'a> {
    world: &'a World,
    client: &'a ClientView,
    engine: &'a EngineInfo,
}

fn lease(l: &Lent) -> mlua::Result<Lease<'_>> {
    let (w, c, e) = (l.world.get(), l.client.get(), l.engine.get());
    if w.is_null() || c.is_null() || e.is_null() {
        return Err(mlua::Error::runtime("view is only available while the UI is building or handling input"));
    }
    // SAFETY: set only for the duration of `UiVm::build`/`call_handler`,
    // which hold shared borrows of all three for that whole time.
    Ok(unsafe { Lease { world: &*w, client: &*c, engine: &*e } })
}

#[derive(Clone)]
pub struct Mount {
    pub layer: String,
    pub id: String,
    pub order: i32,
    /// Within a region: "start" (top/left) or "end" (bottom/right).
    pub align: String,
    pub owner: Rc<str>,
}

struct Comp {
    owner: Rc<str>,
    func: Function,
}

#[derive(Default)]
struct Registry {
    current: String,
    comps: HashMap<String, Comp>,
    mounts: Vec<Mount>,
    extends: HashMap<String, Vec<(Rc<str>, Value)>>,
    replaces: HashMap<String, Vec<(Rc<str>, Function)>>,
    wraps: HashMap<String, Vec<(Rc<str>, Function)>>,
    removes: HashMap<String, Vec<Rc<str>>>,
    actions: Vec<UiAction>,
    state: HashMap<String, Value>,
}

/// A loaded mod: id, directory, and which mods it may `require`.
#[derive(Clone)]
pub struct ModDir {
    pub id: String,
    pub dir: PathBuf,
    pub deps: Vec<String>,
}

pub struct UiVm {
    lua: Lua,
    reg: Rc<RefCell<Registry>>,
    lent: Rc<Lent>,
    /// When the running call must stop (see `CALL_DEADLINE`); None outside calls.
    deadline: Rc<Cell<Option<Instant>>>,
    /// Load problems and operation conflicts, reported once.
    pub warnings: Vec<String>,
    /// Component errors seen at run time (deduplicated).
    pub errors: Vec<String>,
    /// Microseconds spent in each mod's UI code, smoothed.
    pub mod_time: HashMap<String, f64>,
    seen_ids: RefCell<HashSet<String>>,
    unknown_checked: bool,
}

/// Longest a single component build or handler may run. UI code is client
/// only, so a wall-clock limit is fine here (the sim VM counts steps
/// instead). Generous: a frame's whole budget is a few milliseconds.
pub const CALL_DEADLINE: std::time::Duration = std::time::Duration::from_millis(250);

/// The mod whose UI code is calling: the chunk name of the nearest Luau
/// frame ("@weather/ui/devtools.luau").
fn ui_calling_mod(lua: &Lua) -> Option<String> {
    (1..16).find_map(|level| {
        lua.inspect_stack(level, |d| {
            let src = d.source().source?.to_string();
            let (mod_id, path) = src.strip_prefix('@')?.split_once('/')?;
            path.starts_with("ui/").then(|| mod_id.to_string())
        })
        .flatten()
    })
}

fn rt(e: impl std::fmt::Display) -> mlua::Error {
    mlua::Error::runtime(e.to_string())
}

impl UiVm {
    pub fn load(mods: &[ModDir]) -> UiVm {
        let lua = Lua::new();
        // Level 2 inlines small local functions and unrolls constant loops;
        // debug level 1 keeps line numbers in error boxes.
        lua.set_compiler(mlua::chunk::Compiler::new().set_optimization_level(2).set_debug_level(1));
        // No memory limit here, unlike the sim VM: measured, it made UI
        // rebuilds 45% slower (0.84 -> 1.25 ms), and a runaway UI mod only
        // hurts this player's client. The deadline below stops endless loops.
        let deadline: Rc<Cell<Option<Instant>>> = Rc::default();
        {
            let deadline = deadline.clone();
            let ticks = Cell::new(0u32);
            lua.set_interrupt(move |_| {
                // Checking the clock is cheap but not free: every 1024 steps.
                let n = ticks.get().wrapping_add(1);
                ticks.set(n);
                if n.is_multiple_of(1024) {
                    if let Some(d) = deadline.get() {
                        if Instant::now() > d {
                            return Err(mlua::Error::runtime(format!(
                                "stopped: ran longer than {} ms (an endless loop?)",
                                CALL_DEADLINE.as_millis()
                            )));
                        }
                    }
                }
                Ok(mlua::VmState::Continue)
            });
        }
        let reg = Rc::new(RefCell::new(Registry::default()));
        let lent = Rc::new(Lent {
            world: Cell::new(std::ptr::null()),
            client: Cell::new(std::ptr::null()),
            engine: Cell::new(std::ptr::null()),
        });
        let mut vm = UiVm {
            lua,
            reg,
            lent,
            deadline,
            warnings: Vec::new(),
            errors: Vec::new(),
            mod_time: HashMap::new(),
            seen_ids: RefCell::new(HashSet::new()),
            unknown_checked: false,
        };
        if let Err(e) = vm.install(mods) {
            vm.warnings.push(format!("UI API setup failed: {e}"));
            return vm;
        }
        // Sandbox: every table reachable from globals becomes read-only, and
        // each script writes its own globals into a private environment.
        if let Err(e) = vm.lua.sandbox(true) {
            vm.warnings.push(format!("UI sandbox failed: {e}"));
        }
        for m in mods {
            let dir = m.dir.join("ui");
            let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
                .map(|rd| {
                    rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "luau")).collect()
                })
                .unwrap_or_default();
            files.sort();
            for f in files {
                let name = f.file_stem().unwrap().to_string_lossy().to_string();
                let r: mlua::Result<Value> = vm
                    .lua
                    .globals()
                    .get::<Function>("require")
                    .and_then(|req| req.call(format!("@{}/ui/{}", m.id, name)));
                if let Err(e) = r {
                    vm.warnings.push(format!("{}/ui/{}.luau: {}", m.id, name, first_line(&e.to_string())));
                }
            }
        }
        vm.report_conflicts();
        vm
    }

    fn install(&self, mods: &[ModDir]) -> mlua::Result<()> {
        let lua = &self.lua;
        let g = lua.globals();
        g.set("os", Value::Nil)?;
        g.set("io", Value::Nil)?;

        // ---- require: "@<mod>/ui/<file>", limited to declared dependencies.
        let loaded: Rc<RefCell<HashMap<String, Value>>> = Rc::default();
        let loading: Rc<RefCell<HashSet<String>>> = Rc::default();
        let dirs: HashMap<String, ModDir> = mods.iter().map(|m| (m.id.clone(), m.clone())).collect();
        let reg = self.reg.clone();
        let require = lua.create_function(move |lua, path: String| {
            if let Some(v) = loaded.borrow().get(&path) {
                return Ok(v.clone());
            }
            let rest =
                path.strip_prefix('@').ok_or_else(|| rt(format!("require '{path}': use \"@<mod>/ui/<file>\"")))?;
            let (mod_id, file) = rest.split_once('/').ok_or_else(|| rt(format!("require '{path}': missing file")))?;
            let file =
                file.strip_prefix("ui/").ok_or_else(|| rt(format!("require '{path}': UI modules live under ui/")))?;
            let caller = reg.borrow().current.clone();
            let target = dirs.get(mod_id).ok_or_else(|| rt(format!("require '{path}': no mod '{mod_id}'")))?;
            if !caller.is_empty() && caller != mod_id {
                let caller_deps = dirs.get(&caller).map(|m| m.deps.clone()).unwrap_or_default();
                if !caller_deps.iter().any(|d| d == mod_id) {
                    return Err(rt(format!("mod '{caller}' requires '{path}' but doesn't depend on '{mod_id}'")));
                }
            }
            if !loading.borrow_mut().insert(path.clone()) {
                return Err(rt(format!("require cycle through '{path}'")));
            }
            let src_path = target.dir.join("ui").join(format!("{file}.luau"));
            let src = std::fs::read_to_string(&src_path).map_err(|e| rt(format!("{}: {e}", src_path.display())))?;
            let prev = std::mem::replace(&mut reg.borrow_mut().current, mod_id.to_string());
            let env = lua.create_table()?;
            let mt = lua.create_table()?;
            mt.set("__index", lua.globals())?;
            env.set_metatable(Some(mt))?;
            // Globals are read-only (sandboxed), so the module's environment
            // is safe: Luau caches imports like `kit.label` and takes its
            // fastcall and fast-iteration paths.
            env.set_safeenv(true);
            let result =
                lua.load(&src).set_name(format!("@{mod_id}/ui/{file}.luau")).set_environment(env).eval::<Value>();
            reg.borrow_mut().current = prev;
            loading.borrow_mut().remove(&path);
            let v = result?;
            loaded.borrow_mut().insert(path, v.clone());
            Ok(v)
        })?;
        g.set("require", require)?;

        // ---- ui: node constructors, registration, operations, state.
        let ui = lua.create_table()?;
        for kind in ["row", "col", "text", "spacer", "scroll", "anchored"] {
            let f = lua.create_function(move |lua, v: Value| {
                let t = match v {
                    Value::Table(t) => t,
                    Value::Nil => lua.create_table()?,
                    other => {
                        let t = lua.create_table()?;
                        t.set(1, other)?;
                        t
                    }
                };
                t.set("kind", kind)?;
                Ok(t)
            })?;
            ui.set(kind, f)?;
        }
        ui.set(
            "slot",
            lua.create_function(|lua, id: String| {
                let t = lua.create_table()?;
                t.set("kind", "slot")?;
                t.set("id", id)?;
                Ok(t)
            })?,
        )?;
        let r = self.reg.clone();
        ui.set(
            "define",
            lua.create_function(move |_, (id, func): (String, Function)| {
                let mut reg = r.borrow_mut();
                let owner: Rc<str> = reg.current.as_str().into();
                if let Some(prev) = reg.comps.get(&id) {
                    if *prev.owner != *owner {
                        return Err(rt(format!(
                            "component '{id}' is already defined by '{}'; use ui.replace to change it",
                            prev.owner
                        )));
                    }
                }
                reg.comps.insert(id, Comp { owner, func });
                Ok(())
            })?,
        )?;
        let r = self.reg.clone();
        ui.set(
            "mount",
            lua.create_function(move |_, (layer, id, opts): (String, String, Option<Table>)| {
                const LAYERS: &[&str] = &["top", "bottom", "left", "right", "anchored", "cursor", "modal", "windows"];
                if !LAYERS.contains(&layer.as_str()) {
                    return Err(rt(format!("unknown layer '{layer}' (one of {})", LAYERS.join(", "))));
                }
                let order = opts.as_ref().and_then(|o| o.get::<Option<i32>>("order").ok().flatten()).unwrap_or(0);
                let align = opts
                    .as_ref()
                    .and_then(|o| o.get::<Option<String>>("align").ok().flatten())
                    .unwrap_or("start".into());
                let mut reg = r.borrow_mut();
                let owner: Rc<str> = reg.current.as_str().into();
                reg.mounts.push(Mount { layer, id, order, align, owner });
                Ok(())
            })?,
        )?;
        let r = self.reg.clone();
        ui.set(
            "extend",
            lua.create_function(move |_, (id, v): (String, Value)| {
                let mut reg = r.borrow_mut();
                let owner: Rc<str> = reg.current.as_str().into();
                reg.extends.entry(id).or_default().push((owner, v));
                Ok(())
            })?,
        )?;
        let r = self.reg.clone();
        ui.set(
            "replace",
            lua.create_function(move |_, (id, f): (String, Function)| {
                let mut reg = r.borrow_mut();
                let owner: Rc<str> = reg.current.as_str().into();
                reg.replaces.entry(id).or_default().push((owner, f));
                Ok(())
            })?,
        )?;
        let r = self.reg.clone();
        ui.set(
            "wrap",
            lua.create_function(move |_, (id, f): (String, Function)| {
                let mut reg = r.borrow_mut();
                let owner: Rc<str> = reg.current.as_str().into();
                reg.wraps.entry(id).or_default().push((owner, f));
                Ok(())
            })?,
        )?;
        let r = self.reg.clone();
        ui.set(
            "remove",
            lua.create_function(move |_, id: String| {
                let mut reg = r.borrow_mut();
                let owner: Rc<str> = reg.current.as_str().into();
                reg.removes.entry(id).or_default().push(owner);
                Ok(())
            })?,
        )?;
        let r = self.reg.clone();
        ui.set(
            "state",
            lua.create_function(move |_, (key, default): (String, Value)| {
                Ok(r.borrow().state.get(&key).cloned().unwrap_or(default))
            })?,
        )?;
        let r = self.reg.clone();
        ui.set(
            "set_state",
            lua.create_function(move |_, (key, v): (String, Value)| {
                r.borrow_mut().state.insert(key, v);
                Ok(())
            })?,
        )?;
        g.set("ui", ui)?;

        // ---- act: queue actions for the client.
        let act = lua.create_table()?;
        macro_rules! act {
            ($name:literal, $ty:ty, |$args:pat_param| $action:expr) => {{
                let r = self.reg.clone();
                act.set(
                    $name,
                    lua.create_function(move |_, $args: $ty| {
                        r.borrow_mut().actions.push($action);
                        Ok(())
                    })?,
                )?;
            }};
        }
        act!("select", Option<u64>, |id| UiAction::Select(id.and_then(Entity::from_bits)));
        act!("focus", u64, |id| match Entity::from_bits(id) {
            Some(e) => UiAction::Focus(e),
            None => return Err(rt("bad entity id")),
        });
        act!("tool", String, |key| UiAction::Tool(key));
        act!("stuff", String, |id| UiAction::Stuff(id));
        act!("speed", u32, |s| UiAction::Speed(s));
        act!("toggle_pause", (), |_a| UiAction::TogglePause);
        act!("draft", (u64, bool), |(id, on)| match Entity::from_bits(id) {
            Some(e) => UiAction::Draft(e, on),
            None => return Err(rt("bad entity id")),
        });
        act!("cycle_overlay", (), |_a| UiAction::CycleOverlay);
        act!("set_overlay", Option<usize>, |i| UiAction::SetOverlay(i.map(|i| i.saturating_sub(1))));
        act!("toggle_profiler", (), |_a| UiAction::ToggleProfiler);
        act!("toggle_devtools", (), |_a| UiAction::ToggleDevtools);
        act!("toggle_outlines", (), |_a| UiAction::ToggleOutlines);
        // Devtools: run the sim forward (hours of game time).
        act!("advance", f64, |h| UiAction::Advance(h.clamp(0.0, 24.0 * 60.0)));
        // Send an event to this mod's own sim scripts: "<mod>:<name>". The mod
        // is the one whose UI code calls it (from its chunk name), so a mod
        // can't speak for another.
        {
            let r = self.reg.clone();
            act.set(
                "send",
                lua.create_function(move |lua, (name, data): (String, Option<Table>)| {
                    let me = ui_calling_mod(lua).unwrap_or_default();
                    if me.is_empty() || !name.starts_with(&format!("{me}:")) || name.len() <= me.len() + 1 {
                        return Err(rt(format!(
                            "mod '{me}' can only send its own events, named \"{me}:<event>\" (got \"{name}\")"
                        )));
                    }
                    let data = match data {
                        Some(t) => rim_sim::data::from_lua(&Value::Table(t), &name, 0).map_err(rt)?,
                        None => None,
                    };
                    r.borrow_mut().actions.push(UiAction::Send(name, data));
                    Ok(())
                })?,
            )?;
        }
        g.set("act", act)?;

        g.set("view", self.view_api()?)?;
        Ok(())
    }

    /// Every function in `ui`, `act` and `view` ("view.tick"), sorted: tests
    /// check them against the declarations in `api.rs`.
    pub fn api_names(&self) -> Vec<String> {
        let mut v = Vec::new();
        for global in ["ui", "act", "view"] {
            if let Ok(t) = self.lua.globals().get::<Table>(global) {
                for (k, _) in t.pairs::<String, Value>().flatten() {
                    v.push(format!("{global}.{k}"));
                }
            }
        }
        v.sort();
        v
    }

    /// `view`: read-only questions about the world and the client.
    fn view_api(&self) -> mlua::Result<Table> {
        let lua = &self.lua;
        let view = lua.create_table()?;
        macro_rules! view {
            ($name:literal, $ty:ty, |$lua:ident, $l:ident, $args:pat_param| $body:expr) => {{
                let lent = self.lent.clone();
                view.set(
                    $name,
                    lua.create_function(move |$lua, $args: $ty| {
                        let $l = lease(&lent)?;
                        $body
                    })?,
                )?;
            }};
        }

        view!("tick", (), |_lua, l, _a| Ok(l.world.tick));
        view!("ticks_per_day", (), |_lua, _l, _a| Ok(rim_sim::TICKS_PER_DAY));
        view!("day", (), |_lua, l, _a| Ok(l.world.day() + 1));
        view!("hour", (), |_lua, l, _a| Ok(l.world.hour()));
        view!("clock", (), |_lua, l, _a| {
            let h = l.world.hour();
            Ok(format!("{:02}:{:02}", h as u32, (h.fract() * 60.0) as u32))
        });
        view!("paused", (), |_lua, l, _a| Ok(l.client.paused));
        view!("speed", (), |_lua, l, _a| Ok(l.client.speed));
        view!("wealth", (), |_lua, l, _a| Ok(l.world.wealth));
        view!("time", (), |_lua, l, _a| Ok(l.client.time));
        view!("colony_lost", (), |_lua, l, _a| Ok(l.world.colony_lost));
        view!("selected", (), |_lua, l, _a| Ok(l.client.selected.map(|e| e.to_bits().get())));
        view!("show_profiler", (), |_lua, l, _a| Ok(l.client.show_profiler));
        view!("show_devtools", (), |_lua, l, _a| Ok(l.client.show_devtools));
        view!("hint", (), |_lua, l, _a| Ok(l.client.hint.clone()));
        view!("screen", (), |_lua, l, _a| Ok((l.client.screen.0, l.client.screen.1)));

        view!("colonists", (), |lua, l, _a| {
            let t = lua.create_table()?;
            for e in l.world.colonists() {
                if let Some(p) = pawn_table(lua, l.world, l.client, e)? {
                    t.push(p)?;
                }
            }
            Ok(t)
        });
        // How many living pawns of a faction ("player", "hostile", "wild").
        view!("count_pawns", String, |_lua, l, faction| {
            Ok(l.world
                .pawns
                .iter()
                .filter(|&&e| {
                    l.world.ecs.get::<&Pawn>(e).is_ok_and(|p| p.active && !p.dead && p.faction.name() == faction)
                })
                .count())
        });
        view!("pawn", u64, |lua, l, id| match Entity::from_bits(id) {
            Some(e) => pawn_table(lua, l.world, l.client, e),
            None => Ok(None),
        });
        // Pawns on screen, for anchored labels.
        view!("visible_pawns", (), |lua, l, _a| {
            let t = lua.create_table()?;
            let (sw, sh) = l.client.screen;
            for &e in &l.world.pawns {
                let Ok(p) = l.world.ecs.get::<&Pawn>(e) else { continue };
                if !p.active || p.dead {
                    continue;
                }
                let (x, y) = crate::view::pawn_screen(&p, l.client);
                let m = l.client.cam.2 * 2.0;
                if x < -m || y < -m || x > sw + m || y > sh + m {
                    continue;
                }
                let cd = l.world.defs.creature(p.def);
                let row = lua.create_table()?;
                row.set("id", e.to_bits().get())?;
                row.set("name", if cd.intelligent { p.name.as_str() } else { cd.label.as_str() })?;
                row.set("faction", p.faction.name())?;
                row.set("intelligent", cd.intelligent)?;
                row.set("asleep", p.asleep)?;
                row.set("selected", l.client.selected == Some(e))?;
                row.set("hovered", l.client.hover_pawn == Some(e))?;
                // Logical pixels, like every size a component writes.
                row.set("radius", cd.size * l.client.cam.2 / l.client.scale.max(0.1))?;
                t.push(row)?;
            }
            Ok(t)
        });
        view!("messages", usize, |lua, l, max| {
            let t = lua.create_table()?;
            for m in l.world.messages.iter().rev().take(max) {
                let age = l.world.tick.saturating_sub(m.tick) as f64 / TICKS_PER_DAY as f64;
                let row = lua.create_table()?;
                row.set("text", m.text.as_str())?;
                row.set(
                    "kind",
                    match m.kind {
                        MsgKind::Info => "info",
                        MsgKind::Good => "good",
                        MsgKind::Threat => "threat",
                        MsgKind::Bad => "bad",
                    },
                )?;
                row.set("age", age)?;
                t.push(row)?;
            }
            Ok(t)
        });
        // Recent world events (joins, deaths...), newest last.
        view!("events", u64, |lua, l, since| {
            let t = lua.create_table()?;
            for (tick, kind, id, name) in &l.world.recent_events {
                if *tick >= since {
                    let row = lua.create_table()?;
                    row.set("tick", *tick)?;
                    row.set("kind", *kind)?;
                    row.set("id", id.to_bits().get())?;
                    row.set("name", name.as_str())?;
                    t.push(row)?;
                }
            }
            Ok(t)
        });
        view!("fields", (), |lua, l, _a| {
            let t = lua.create_table()?;
            for (i, fd) in l.world.defs.fields.iter().enumerate() {
                let row = lua.create_table()?;
                row.set("index", i + 1)?;
                row.set("id", fd.id.as_str())?;
                row.set("label", fd.label.as_str())?;
                row.set("unit", fd.unit.as_str())?;
                row.set("hud", fd.hud)?;
                row.set("overlay", fd.overlay)?;
                row.set("ambient", l.world.fields.ambient(i))?;
                row.set("shown", l.client.overlay == Some(i))?;
                t.push(row)?;
            }
            Ok(t)
        });
        view!("overlay", (), |_lua, l, _a| Ok(l.client.overlay.map(|i| l.world.defs.fields[i].label.clone())));
        // The calendar: { year, season, day, day_of_year, year_days }.
        view!("date", (), |lua, l, _a| {
            let w = l.world;
            let t = lua.create_table()?;
            t.set("year", w.year() + 1)?;
            t.set("season", w.season())?;
            t.set("season_index", w.season_index() + 1)?;
            t.set("day", w.day_of_season())?;
            t.set("day_of_year", w.day_of_year() + 1)?;
            t.set("year_days", w.defs.calendar.year_days)?;
            Ok(t)
        });
        // A field's outdoor value, or nil for an unknown field.
        view!("ambient", String, |lua, l, id| {
            let from = ui_calling_mod(lua).unwrap_or_default();
            Ok(l.world.defs.resolve("field", &id, &from).ok().map(|f| l.world.fields.ambient(f as usize)))
        });
        // Each part of a field's outdoor value: { {label, value}, ... }.
        view!("explain", String, |lua, l, id| {
            let t = lua.create_table()?;
            if let Ok(f) = l.world.defs.resolve("field", &id, &ui_calling_mod(lua).unwrap_or_default()) {
                for (label, v) in l.world.fields.explain_ambient(&l.world.defs, f as usize) {
                    let row = lua.create_table()?;
                    row.set("label", label)?;
                    row.set("value", v)?;
                    t.push(row)?;
                }
            }
            Ok(t)
        });
        // Data a sim script stored with rim.set_data (a copy), or nil.
        view!("data", String, |lua, l, key| match l.world.data.get(&key) {
            Some(d) => rim_sim::data::to_lua(lua, d),
            None => Ok(Value::Nil),
        });
        view!("tools", (), |lua, l, _a| {
            let t = lua.create_table()?;
            for tool in &l.client.tools {
                let row = lua.create_table()?;
                row.set("key", tool.key.as_str())?;
                row.set("label", tool.label.as_str())?;
                row.set("color", format!("#{:02x}{:02x}{:02x}", tool.color[0], tool.color[1], tool.color[2]))?;
                row.set("active", tool.active)?;
                t.push(row)?;
            }
            Ok(t)
        });
        // Materials the active build tool could use: what you have, what
        // you would get. Empty unless a stuff buildable is selected.
        view!("stuff", (), |lua, l, _a| {
            let t = lua.create_table()?;
            for m in &l.client.stuff {
                let row = lua.create_table()?;
                row.set("id", m.id.as_str())?;
                row.set("label", m.label.as_str())?;
                row.set("color", format!("#{:02x}{:02x}{:02x}", m.color[0], m.color[1], m.color[2]))?;
                row.set("have", m.have)?;
                row.set("active", m.active)?;
                row.set("hp", m.hp)?;
                row.set("work", m.work)?;
                t.push(row)?;
            }
            Ok(t)
        });
        view!("hover", (), |lua, l, _a| hover_table(lua, l.world, l.client));
        view!("profile", (), |lua, l, _a| {
            let t = lua.create_table()?;
            for (name, us) in &l.client.profile {
                let row = lua.create_table()?;
                row.set("name", name.as_str())?;
                row.set("us", *us)?;
                row.set("mod", name.starts_with("mod:"))?;
                t.push(row)?;
            }
            Ok(t)
        });
        view!("stats", (), |lua, l, _a| lua.create_sequence_from(l.client.stats.iter().cloned()));
        view!("mods", (), |lua, l, _a| {
            let t = lua.create_table()?;
            for (id, version, name) in &l.client.mods {
                let row = lua.create_table()?;
                row.set("id", id.as_str())?;
                row.set("version", version.as_str())?;
                row.set("name", name.as_str())?;
                t.push(row)?;
            }
            Ok(t)
        });
        view!("warnings", (), |lua, l, _a| lua.create_sequence_from(l.client.warnings.iter().cloned()));
        view!("ui_stats", (), |lua, l, _a| {
            let t = lua.create_table()?;
            let e = l.engine;
            t.set("font", e.font.as_str())?;
            t.set("build_us", e.build_us)?;
            t.set("layout_us", e.layout_us)?;
            t.set("paint_us", e.paint_us)?;
            t.set("nodes", e.nodes)?;
            t.set("layouts", e.layouts)?;
            Ok(t)
        });
        view!("inspect", (), |lua, l, _a| {
            let Some(i) = &l.engine.inspect else { return Ok(Value::Nil) };
            let t = lua.create_table()?;
            t.set("id", i.id.as_str())?;
            t.set("owner", i.owner.as_str())?;
            t.set("kind", i.kind.as_str())?;
            t.set("path", i.path.as_str())?;
            t.set("x", i.rect[0])?;
            t.set("y", i.rect[1])?;
            t.set("w", i.rect[2])?;
            t.set("h", i.rect[3])?;
            Ok(Value::Table(t))
        });
        view!("ui_tree", (), |lua, l, _a| {
            let t = lua.create_table()?;
            for (depth, kind, id, owner) in &l.engine.tree {
                let row = lua.create_table()?;
                row.set("depth", *depth)?;
                row.set("kind", kind.as_str())?;
                row.set("id", id.as_str())?;
                row.set("owner", owner.as_str())?;
                t.push(row)?;
            }
            Ok(t)
        });
        view!("outlines", (), |_lua, l, _a| Ok(l.engine.outlines));
        Ok(view)
    }

    fn report_conflicts(&mut self) {
        let reg = self.reg.borrow();
        let mut ids: Vec<&String> = reg.replaces.keys().chain(reg.removes.keys()).collect();
        ids.sort();
        ids.dedup();
        for id in ids {
            let mut who: Vec<String> = Vec::new();
            for (o, _) in reg.replaces.get(id).into_iter().flatten() {
                who.push(o.to_string());
            }
            for o in reg.removes.get(id).into_iter().flatten() {
                who.push(o.to_string());
            }
            let first = who.first().cloned().unwrap_or_default();
            if who.iter().any(|w| *w != first) {
                let last = who.last().unwrap();
                self.warnings.push(format!(
                    "UI conflict: '{id}' replaced or removed by {} ('{last}' wins by load order)",
                    who.iter().map(|w| format!("'{w}'")).collect::<Vec<_>>().join(" and ")
                ));
            }
        }
    }

    /// Mounted components, in registration order.
    pub fn mounts(&self) -> Vec<Mount> {
        self.reg.borrow().mounts.clone()
    }

    fn lend<R>(&self, world: &World, client: &ClientView, engine: &EngineInfo, f: impl FnOnce() -> R) -> R {
        self.lent.world.set(world);
        self.lent.client.set(client);
        self.lent.engine.set(engine);
        let r = f();
        self.lent.world.set(std::ptr::null());
        self.lent.client.set(std::ptr::null());
        self.lent.engine.set(std::ptr::null());
        r
    }

    /// Build every mounted component. Returns (mount, tree) pairs.
    pub fn build(
        &mut self,
        world: &World,
        client: &ClientView,
        engine: &EngineInfo,
        theme: &Theme,
    ) -> Vec<(Mount, Node)> {
        let mounts = self.mounts();
        let view: Table = self.lua.globals().get("view").unwrap();
        let mut times: HashMap<Rc<str>, f64> = HashMap::new();
        let mut errors = Vec::new();
        let out = self.lend(world, client, engine, || {
            let mut b = Builder { vm: self, theme, view: &view, times: &mut times, errors: &mut errors };
            mounts
                .into_iter()
                .filter_map(|m| {
                    let key = key_for(0, 0, Some(&m.id));
                    let owner = m.owner.clone();
                    b.expand_slot(&m.id, key, &owner).map(|n| (m, n))
                })
                .collect::<Vec<_>>()
        });
        for (m, us) in times {
            let e = self.mod_time.entry(m.to_string()).or_insert(us);
            *e = *e * 0.9 + us * 0.1;
        }
        for e in errors {
            if !self.errors.contains(&e) {
                eprintln!("rim_ui: {e}");
                self.errors.push(e);
            }
        }
        if !self.unknown_checked {
            self.unknown_checked = true;
            let seen = self.seen_ids.borrow();
            let reg = self.reg.borrow();
            let mut warn = Vec::new();
            for (op, ids) in [
                ("extend", reg.extends.keys().collect::<Vec<_>>()),
                ("replace", reg.replaces.keys().collect()),
                ("wrap", reg.wraps.keys().collect()),
                ("remove", reg.removes.keys().collect()),
            ] {
                for id in ids {
                    if !seen.contains(id) && !reg.comps.contains_key(id) {
                        warn.push(format!("ui.{op}('{id}'): no component or node has that id"));
                    }
                }
            }
            drop(reg);
            drop(seen);
            self.warnings.extend(warn);
        }
        out
    }

    /// Run a click handler, then collect whatever actions it queued.
    pub fn call_handler(&mut self, f: &Function, world: &World, client: &ClientView, engine: &EngineInfo) {
        self.deadline.set(Some(Instant::now() + CALL_DEADLINE));
        let r = self.lend(world, client, engine, || f.call::<()>(()));
        self.deadline.set(None);
        if let Err(e) = r {
            let e = format!("handler: {}", first_line(&e.to_string()));
            if !self.errors.contains(&e) {
                self.errors.push(e);
            }
        }
    }

    pub fn take_actions(&mut self) -> Vec<UiAction> {
        std::mem::take(&mut self.reg.borrow_mut().actions)
    }

    /// Owner of a mounted/defined component, for devtools.
    pub fn component_owner(&self, id: &str) -> Option<String> {
        self.reg.borrow().comps.get(id).map(|c| c.owner.to_string())
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").to_string()
}

struct Builder<'a> {
    vm: &'a UiVm,
    theme: &'a Theme,
    view: &'a Table,
    times: &'a mut HashMap<Rc<str>, f64>,
    errors: &'a mut Vec<String>,
}

impl Builder<'_> {
    fn call(&mut self, owner: &Rc<str>, f: &Function, args: impl mlua::IntoLuaMulti) -> Result<Value, String> {
        let t = Instant::now();
        self.vm.deadline.set(Some(t + CALL_DEADLINE));
        let r = f.call::<Value>(args).map_err(|e| first_line(&e.to_string()));
        self.vm.deadline.set(None);
        *self.times.entry(owner.clone()).or_default() += t.elapsed().as_secs_f64() * 1e6;
        r
    }

    fn fail(&mut self, owner: &Rc<str>, key: u64, what: &str, err: String) -> Node {
        self.errors.push(format!("{owner}: {what}: {err}"));
        error_node(self.theme, owner.clone(), key, &format!("{owner} · {what}"), &err)
    }

    /// Expand a component by id, applying replace, wrap and remove.
    fn expand_slot(&mut self, id: &str, key: u64, owner: &Rc<str>) -> Option<Node> {
        self.vm.seen_ids.borrow_mut().insert(id.to_string());
        let (base, wraps) = {
            let reg = self.vm.reg.borrow();
            if reg.removes.contains_key(id) {
                return None;
            }
            let base = match reg.replaces.get(id).and_then(|v| v.last()) {
                Some((o, f)) => Some((o.clone(), f.clone())),
                None => reg.comps.get(id).map(|c| (c.owner.clone(), c.func.clone())),
            };
            (base, reg.wraps.get(id).cloned().unwrap_or_default())
        };
        let Some((mut who, f)) = base else {
            return Some(self.fail(owner, key, id, "no component with this id".into()));
        };
        let mut v = match self.call(&who, &f, self.view.clone()) {
            Ok(v) => v,
            Err(e) => return Some(self.fail(&who, key, id, e)),
        };
        for (wo, wf) in wraps {
            v = match self.call(&wo, &wf, (v, self.view.clone())) {
                Ok(v) => v,
                Err(e) => return Some(self.fail(&wo, key, &format!("wrap {id}"), e)),
            };
            who = wo;
        }
        match v {
            Value::Nil => None,
            Value::Table(t) => {
                // The component root answers to the component's id, and to
                // its own id if it set one.
                let own: Option<String> = t.get("id").ok().flatten();
                if own.is_none() {
                    let _ = t.raw_set("id", id);
                }
                let mut n = self.convert(&t, key, &who, Some(id))?;
                if own.as_deref().is_some_and(|o| o != id) {
                    n.aka = Some(Rc::from(id));
                }
                Some(n)
            }
            _ => Some(self.fail(&who, key, id, "component must return a node table or nil".into())),
        }
    }

    /// Convert a node table and its children. `applied` is the id whose
    /// operations were already applied by `expand_slot`.
    fn convert(&mut self, t: &Table, key: u64, owner: &Rc<str>, applied: Option<&str>) -> Option<Node> {
        let kind: Option<String> = t.get("kind").ok().flatten();
        let id: Option<String> = t.get("id").ok().flatten();
        if kind.as_deref() == Some("slot") {
            let id = id?;
            return self.expand_slot(&id, key, owner);
        }
        // Operations on inner nodes that carry an id.
        if let Some(id) = id.as_deref().filter(|i| Some(*i) != applied) {
            self.vm.seen_ids.borrow_mut().insert(id.to_string());
            let (removed, replace, wraps) = {
                let reg = self.vm.reg.borrow();
                (
                    reg.removes.contains_key(id),
                    reg.replaces.get(id).and_then(|v| v.last()).cloned(),
                    reg.wraps.get(id).cloned().unwrap_or_default(),
                )
            };
            if removed {
                return None;
            }
            if replace.is_some() || !wraps.is_empty() {
                let mut v = Value::Table(t.clone());
                let mut who = owner.clone();
                if let Some((o, f)) = replace {
                    v = match self.call(&o, &f, self.view.clone()) {
                        Ok(v) => v,
                        Err(e) => return Some(self.fail(&o, key, id, e)),
                    };
                    who = o;
                }
                for (wo, wf) in wraps {
                    v = match self.call(&wo, &wf, (v, self.view.clone())) {
                        Ok(v) => v,
                        Err(e) => return Some(self.fail(&wo, key, &format!("wrap {id}"), e)),
                    };
                    who = wo;
                }
                return match v {
                    Value::Table(nt) => {
                        if nt.get::<Option<String>>("id").ok().flatten().is_none() {
                            let _ = nt.raw_set("id", id);
                        }
                        self.convert(&nt, key, &who, Some(id))
                    }
                    _ => None,
                };
            }
        }

        let ctx = Ctx { theme: self.theme, owner: owner.clone() };
        let mut node = match node_from_table(&ctx, t, key) {
            Ok(n) => n,
            Err(e) => return Some(self.fail(owner, key, id.as_deref().unwrap_or("node"), e)),
        };
        let mut index = 0;
        for child in t.sequence_values::<Value>() {
            let Ok(Value::Table(c)) = child else { continue };
            let cid: Option<String> = c.get("id").ok().flatten();
            let ck = key_for(key, index, cid.as_deref());
            index += 1;
            if let Some(n) = self.convert(&c, ck, owner, None) {
                node.children.push(n);
            }
        }
        // Children other mods added to this node.
        if let Some(id) = &id {
            let exts = self.vm.reg.borrow().extends.get(id).cloned().unwrap_or_default();
            for (eo, ev) in exts {
                let v = match ev {
                    Value::Function(f) => match self.call(&eo, &f, self.view.clone()) {
                        Ok(v) => v,
                        Err(e) => {
                            node.children.push(self.fail(&eo, key_for(key, index, None), &format!("extend {id}"), e));
                            index += 1;
                            continue;
                        }
                    },
                    v => v,
                };
                if let Value::Table(c) = v {
                    let cid: Option<String> = c.get("id").ok().flatten();
                    let ck = key_for(key, index, cid.as_deref());
                    index += 1;
                    if let Some(n) = self.convert(&c, ck, &eo, None) {
                        node.children.push(n);
                    }
                }
            }
        }
        Some(node)
    }
}

/// A pawn as the UI sees it.
fn pawn_table(lua: &Lua, w: &World, client: &ClientView, e: Entity) -> mlua::Result<Option<Table>> {
    let Ok(p) = w.ecs.get::<&Pawn>(e) else { return Ok(None) };
    if !p.active || p.dead {
        return Ok(None);
    }
    let defs = &w.defs;
    let cd = defs.creature(p.def);
    let t = lua.create_table()?;
    t.set("id", e.to_bits().get())?;
    t.set("name", p.name.as_str())?;
    t.set("label", cd.label.as_str())?;
    t.set("faction", p.faction.name())?;
    t.set("player", p.faction == Faction::Player)?;
    t.set("founder", p.founder)?;
    t.set("drafted", p.drafted)?;
    t.set("asleep", p.asleep)?;
    t.set("hp", p.hp.max(0))?;
    t.set("max_hp", cd.max_hp)?;
    t.set("health", (p.hp.max(0) as f64 / cd.max_hp as f64).clamp(0.0, 1.0))?;
    t.set("job", rim_sim::order::job_text(w, &p))?;
    t.set("selected", client.selected == Some(e))?;
    let needs = lua.create_table()?;
    for &(nid, v) in &p.needs {
        let nd = defs.need(nid);
        let row = lua.create_table()?;
        row.set("id", nd.id.as_str())?;
        row.set("label", nd.label.as_str())?;
        row.set("value", v as f64 / NEED_MAX as f64)?;
        row.set("color", format!("#{:02x}{:02x}{:02x}", nd.rgb[0], nd.rgb[1], nd.rgb[2]))?;
        row.set("low", v < (nd.seek_below * NEED_MAX as f64) as i32 && nd.satisfier != Satisfier::Rest)?;
        needs.push(row)?;
    }
    t.set("needs", needs)?;
    Ok(Some(t))
}

/// What's under the cursor, for the hover readout.
fn hover_table(lua: &Lua, w: &World, client: &ClientView) -> mlua::Result<Value> {
    let Some(tp) = client.hover_cell else { return Ok(Value::Nil) };
    if !w.map.inb(tp) {
        return Ok(Value::Nil);
    }
    let i = w.map.idx(tp);
    let t = lua.create_table()?;
    t.set("x", tp.x)?;
    t.set("y", tp.y)?;
    t.set("terrain", w.defs.terrain[w.map.terrain[i] as usize].label.as_str())?;
    t.set(
        "shelter",
        match w.map.room_at(tp) {
            Some(r) if r.enclosed() => format!("indoors, room of {} cells", r.cells),
            Some(_) => "outdoors".into(),
            None => String::new(),
        },
    )?;
    let readings = lua.create_table()?;
    // Only fields that vary over the map; the weather readout covers the rest.
    for (fi, fd) in w.defs.fields.iter().enumerate().filter(|(_, fd)| fd.overlay) {
        readings.push(format!("{} {:.0}{}", fd.label, w.fields.value(&w.defs, &w.map, fi, tp), fd.unit))?;
    }
    t.set("readings", readings)?;
    let things = lua.create_table()?;
    for e in [w.map.fixture[i], w.map.item[i], w.map.floor[i]].into_iter().flatten() {
        let Ok(th) = w.ecs.get::<&Thing>(e) else { continue };
        let td = w.defs.thing(th.def);
        let mut s = td.label.clone();
        if let Ok(bp) = w.ecs.get::<&Blueprint>(e) {
            let parts: Vec<String> = bp
                .cost
                .iter()
                .zip(&bp.delivered)
                .map(|(c, d)| format!("{}/{} {}", d, c.1, w.defs.thing(c.0).label))
                .collect();
            s = format!("{s} (blueprint: {})", parts.join(", "));
        } else if th.count > 1 {
            s = format!("{s} x{}", th.count);
        }
        if w.ecs.get::<&Regrow>(e).is_ok() {
            s.push_str(" (regrowing)");
        }
        if let Ok(m) = w.ecs.get::<&rim_sim::world::MadeOf>(e) {
            s = format!("{s} · {}", w.defs.thing(m.0).label);
        }
        if td.category == rim_sim::defs::Category::Building && w.ecs.get::<&Blueprint>(e).is_err() {
            if let Some(max) = w.stat(e, "hp") {
                s = format!("{s} · hp {}/{}", th.hp, max.round() as i64);
            }
        }
        things.push(s)?;
    }
    t.set("things", things)?;
    Ok(Value::Table(t))
}

/// UI script directories for the loaded mods, in load order.
pub fn mod_dirs(mods: &[rim_sim::modloader::ModManifest]) -> Vec<ModDir> {
    mods.iter()
        .map(|m| ModDir {
            id: m.id.clone(),
            dir: m.dir.clone(),
            deps: m.depends.iter().chain(&m.optional).chain(&m.load_after).cloned().collect(),
        })
        .collect()
}

pub fn ui_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir.join("ui"))
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "luau" || e == "toml"))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}
