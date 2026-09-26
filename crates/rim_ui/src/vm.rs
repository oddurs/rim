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
use rim_sim::defs::{DefId, Satisfier};
use rim_sim::hecs::Entity;
use rim_sim::world::{skill_xp, Blueprint, Faction, MsgKind, Pawn, Regrow, Thing, World, NEED_MAX, SKILL_MAX};
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
    pub refresh: Refresh,
}

impl Mount {
    /// One mount's identity: a component may be mounted on more than one layer.
    pub fn key(&self) -> String {
        format!("{}@{}", self.layer, self.id)
    }
}

/// How often a mounted component is rebuilt when nothing else forces it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Refresh {
    /// Every frame: a live readout, an animation.
    Frame,
    /// Twenty times a second: the default.
    #[default]
    Fast,
    /// Four times a second: a top bar, a clock.
    Slow,
}

struct Comp {
    owner: Rc<str>,
    func: Function,
}

/// A window a mod declared with `ui.window`. Sizes are logical pixels: the
/// engine scales them, and saves them unscaled.
#[derive(Clone, Debug)]
pub struct WindowDecl {
    pub id: String,
    pub owner: Rc<str>,
    pub title: String,
    pub w: f32,
    pub h: f32,
    pub resizable: bool,
    /// Open when first seen, before any saved layout says otherwise.
    pub open: bool,
    /// The component shown inside the chrome.
    pub comp: String,
}

/// A named action a mod bound to a key with `ui.bind`.
#[derive(Clone)]
pub struct Bind {
    pub id: String,
    pub owner: Rc<str>,
    /// The default key, normalised: modifiers in order then the key, as
    /// "ctrl+shift+p".
    pub key: String,
    pub label: String,
    pub func: Function,
}

/// "Ctrl+Shift+P", "shift + ctrl+p" and "ctrl+shift+p" are one key.
pub fn normalise_key(s: &str) -> String {
    let parts: Vec<String> = s.split('+').map(|p| p.trim().to_ascii_lowercase()).filter(|p| !p.is_empty()).collect();
    let mut mods: Vec<&str> = Vec::new();
    let mut key = String::new();
    for p in &parts {
        match p.as_str() {
            "ctrl" | "control" | "cmd" | "super" => mods.push("ctrl"),
            "alt" | "option" => mods.push("alt"),
            "shift" => mods.push("shift"),
            _ => key = p.clone(),
        }
    }
    let mut out: Vec<&str> = Vec::new();
    for m in ["ctrl", "alt", "shift"] {
        if mods.contains(&m) {
            out.push(m);
        }
    }
    out.push(&key);
    out.join("+")
}

/// What a handler asked of a window; the engine applies it after the call.
#[derive(Clone, Debug, PartialEq)]
pub enum WindowOp {
    Open(String),
    Close(String),
    Toggle(String),
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
    /// `ui.t` overrides from each mod's `ui/lang.toml`, in load order.
    strings: HashMap<String, String>,
    /// Which mod set each string, for conflict reports.
    strings_by: HashMap<String, String>,
    /// Every key `ui.t` has been asked for, so a test can list them.
    used_strings: HashSet<String>,
    /// Windows in declaration order; a later declaration of an id wins.
    windows: Vec<WindowDecl>,
    window_ops: Vec<WindowOp>,
    /// Which windows are open, as the engine last told us.
    window_open: HashMap<String, bool>,
    /// The function that draws a window's chrome around its body.
    chrome: Option<(Rc<str>, Function)>,
    /// Bound actions in declaration order; a later bind of an id wins.
    binds: Vec<Bind>,
    /// The player's keys for bound ids, from the keybinds file.
    key_overrides: HashMap<String, String>,
    /// A handler asked for a node to take keyboard focus.
    focus_req: Option<String>,
    /// A handler set an input's text; the engine replaces its buffer.
    input_sets: Vec<(String, String)>,
    /// Node ids the scripts write as `id = "..."`. A node can exist only
    /// sometimes (the thing inspector's slot, while a thing is selected), so
    /// a target counts as real if it was seen or is declared here.
    declared_ids: HashSet<String>,
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
        // Strings first, so a script's first `ui.t` already sees them.
        for m in mods {
            let path = m.dir.join("ui").join("lang.toml");
            let Ok(src) = std::fs::read_to_string(&path) else { continue };
            let table: toml::Table = match src.parse() {
                Ok(t) => t,
                Err(e) => {
                    vm.warnings.push(format!("{}/ui/lang.toml: {}", m.id, e.message()));
                    continue;
                }
            };
            let mut reg = vm.reg.borrow_mut();
            for (k, v) in table {
                let Some(text) = v.as_str() else {
                    vm.warnings.push(format!("{}/ui/lang.toml: '{k}' must be a string", m.id));
                    continue;
                };
                if let Some(by) = reg.strings_by.get(&k) {
                    if *by != m.id {
                        vm.warnings.push(format!("UI conflict: string '{k}' set by '{by}' and '{}'", m.id));
                    }
                }
                reg.strings.insert(k.clone(), text.to_string());
                reg.strings_by.insert(k, m.id.clone());
            }
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
            reg.borrow_mut().declared_ids.extend(declared_ids(&src));
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
        for kind in ["row", "col", "text", "spacer", "scroll", "anchored", "grid", "list", "image", "input"] {
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
                const LAYERS: &[&str] =
                    &["top", "bottom", "left", "right", "anchored", "cursor", "modal", "windows", "title"];
                if !LAYERS.contains(&layer.as_str()) {
                    return Err(rt(format!("unknown layer '{layer}' (one of {})", LAYERS.join(", "))));
                }
                let order = opts.as_ref().and_then(|o| o.get::<Option<i32>>("order").ok().flatten()).unwrap_or(0);
                let align = opts
                    .as_ref()
                    .and_then(|o| o.get::<Option<String>>("align").ok().flatten())
                    .unwrap_or("start".into());
                let refresh = match opts.as_ref().and_then(|o| o.get::<Option<String>>("refresh").ok().flatten()) {
                    None => Refresh::Fast,
                    Some(s) => match s.as_str() {
                        "frame" => Refresh::Frame,
                        "fast" => Refresh::Fast,
                        "slow" => Refresh::Slow,
                        other => return Err(rt(format!("unknown refresh '{other}' (frame, fast or slow)"))),
                    },
                };
                let mut reg = r.borrow_mut();
                let owner: Rc<str> = reg.current.as_str().into();
                reg.mounts.push(Mount { layer, id, order, align, owner, refresh });
                Ok(())
            })?,
        )?;
        // ui.window(id, { title, w, h, resizable, open }, component): a
        // window the engine moves, sizes, stacks and remembers. The
        // component is a function (defined under the window's id) or the
        // id of one.
        let r = self.reg.clone();
        ui.set(
            "window",
            lua.create_function(move |_, (id, opts, comp): (String, Table, Value)| {
                let mut reg = r.borrow_mut();
                let owner: Rc<str> = reg.current.as_str().into();
                let comp = match comp {
                    Value::Function(func) => {
                        reg.comps.insert(id.clone(), Comp { owner: owner.clone(), func });
                        id.clone()
                    }
                    Value::String(s) => s.to_str()?.to_string(),
                    _ => return Err(rt("ui.window: the component is a function or a component id")),
                };
                let num = |k: &str, d: f32| opts.get::<Option<f32>>(k).ok().flatten().unwrap_or(d);
                reg.windows.push(WindowDecl {
                    id,
                    owner,
                    title: opts.get::<Option<String>>("title").ok().flatten().unwrap_or_default(),
                    w: num("w", 360.0),
                    h: num("h", 240.0),
                    resizable: opts.get::<Option<bool>>("resizable").ok().flatten().unwrap_or(false),
                    open: opts.get::<Option<bool>>("open").ok().flatten().unwrap_or(false),
                    comp,
                });
                Ok(())
            })?,
        )?;
        for (name, op) in [
            ("open", WindowOp::Open as fn(String) -> WindowOp),
            ("close", WindowOp::Close),
            ("toggle", WindowOp::Toggle),
        ] {
            let r = self.reg.clone();
            ui.set(
                name,
                lua.create_function(move |_, id: String| {
                    r.borrow_mut().window_ops.push(op(id));
                    Ok(())
                })?,
            )?;
        }
        let r = self.reg.clone();
        ui.set(
            "is_open",
            lua.create_function(move |_, id: String| Ok(r.borrow().window_open.get(&id).copied().unwrap_or(false)))?,
        )?;
        // ui.bind(id, { key, label }, fn): a named action reachable from
        // its key and from the command palette. With no key it is the
        // palette's alone, until the player gives it one.
        let r = self.reg.clone();
        ui.set(
            "bind",
            lua.create_function(move |_, (id, opts, func): (String, Table, Function)| {
                let key: String = opts.get::<Option<String>>("key")?.unwrap_or_default();
                let label: String = opts.get::<Option<String>>("label")?.unwrap_or_else(|| id.clone());
                let mut reg = r.borrow_mut();
                let owner: Rc<str> = reg.current.as_str().into();
                reg.binds.push(Bind { id, owner, key: normalise_key(&key), label, func });
                Ok(())
            })?,
        )?;
        // ui.run(id): run a bound action, as its key would.
        let r = self.reg.clone();
        ui.set(
            "run",
            lua.create_function(move |_, id: String| {
                let f = r.borrow().binds.iter().rev().find(|b| b.id == id).map(|b| b.func.clone());
                match f {
                    Some(f) => f.call::<()>(()),
                    None => Err(rt(format!("ui.run: no action '{id}'"))),
                }
            })?,
        )?;
        // ui.set_input(id, text): replace what an input holds, caret at the end.
        let r = self.reg.clone();
        ui.set(
            "set_input",
            lua.create_function(move |_, (id, text): (String, String)| {
                r.borrow_mut().input_sets.push((id, text));
                Ok(())
            })?,
        )?;
        // ui.focus(id): give a node (a text input) the keyboard.
        let r = self.reg.clone();
        ui.set(
            "focus",
            lua.create_function(move |_, id: String| {
                r.borrow_mut().focus_req = Some(id);
                Ok(())
            })?,
        )?;
        let r = self.reg.clone();
        ui.set(
            "window_chrome",
            lua.create_function(move |_, f: Function| {
                let mut reg = r.borrow_mut();
                let owner: Rc<str> = reg.current.as_str().into();
                reg.chrome = Some((owner, f));
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
        // ui.t(key, default): the one door every user-visible string goes
        // through. A language file backs it; until one does, the default.
        let r = self.reg.clone();
        ui.set(
            "t",
            lua.create_function(move |_, (key, default): (String, Option<String>)| {
                let mut reg = r.borrow_mut();
                reg.used_strings.insert(key.clone());
                Ok(reg.strings.get(&key).cloned().or(default).unwrap_or(key))
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
        act!("set_priority", (u64, String, u8), |(id, work, level)| match Entity::from_bits(id) {
            Some(e) => UiAction::SetPriority(e, work, level),
            None => return Err(rt("bad entity id")),
        });
        act!("set_stance", String, |id| UiAction::SetStance(id));
        act!("zone_allow", (u32, String, bool), |(zone, item, on)| UiAction::ZoneAllow(zone, item, on));
        act!("cycle_overlay", (), |_a| UiAction::CycleOverlay);
        act!("set_overlay", Option<usize>, |i| UiAction::SetOverlay(i.map(|i| i.saturating_sub(1))));
        act!("toggle_profiler", (), |_a| UiAction::ToggleProfiler);
        act!("toggle_devtools", (), |_a| UiAction::ToggleDevtools);
        act!("toggle_outlines", (), |_a| UiAction::ToggleOutlines);
        act!("render_scale", f32, |s| match s.is_finite() {
            true => UiAction::RenderScale(s.clamp(0.25, 1.0)),
            false => return Err(rt("act.render_scale: wants a number from 0.25 to 1")),
        });
        // Devtools: run the sim forward (hours of game time).
        act!("advance", f64, |h| UiAction::Advance(h.clamp(0.0, 24.0 * 60.0)));
        act!("load", String, |path| UiAction::Load(path));
        act!("new_colony", (), |_a| UiAction::NewColony);
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

        view!("colonists", Option<usize>, |lua, l, max| {
            let t = lua.create_table()?;
            for e in l.world.colonists().take(max.unwrap_or(usize::MAX)) {
                if let Some(p) = pawn_table(lua, l.world, l.client, e)? {
                    t.raw_push(p)?;
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
        view!("thing", u64, |lua, l, id| match Entity::from_bits(id) {
            Some(e) => thing_table(lua, l.world, e),
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
                let row = lua.create_table_with_capacity(0, 8)?;
                row.raw_set("id", e.to_bits().get())?;
                row.raw_set("name", if cd.intelligent { p.name.as_str() } else { cd.label.as_str() })?;
                row.raw_set("faction", p.faction.name())?;
                row.raw_set("intelligent", cd.intelligent)?;
                row.raw_set("asleep", p.asleep)?;
                row.raw_set("selected", l.client.selected == Some(e))?;
                row.raw_set("hovered", l.client.hover_pawn == Some(e))?;
                // Logical pixels, like every size a component writes.
                row.raw_set("radius", cd.size * l.client.cam.2 / l.client.scale.max(0.1))?;
                t.raw_push(row)?;
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
        view!("work_types", (), |lua, l, _a| {
            let t = lua.create_table()?;
            let defs = &l.world.defs;
            for &w in &defs.work_order {
                let d = &defs.work_types[w as usize];
                let row = lua.create_table()?;
                row.set("id", d.id.as_str())?;
                row.set("label", d.label.as_str())?;
                row.set("icon", d.icon.as_str())?;
                row.set("order", d.order)?;
                row.set("default", d.priority)?;
                t.push(row)?;
            }
            Ok(t)
        });
        view!("priority_levels", (), |_lua, l, _a| Ok(l.world.defs.priority_scale.levels));
        view!("effective", u64, |lua, l, id| {
            let Some(p) = Entity::from_bits(id).and_then(|e| l.world.ecs.get::<&Pawn>(e).ok()) else {
                return Ok(None);
            };
            let t = lua.create_table()?;
            let defs = &l.world.defs;
            for (w, d) in defs.work_types.iter().enumerate() {
                let (value, parts) = rim_sim::rules::explain(defs, &l.world.rules, &p, w as rim_sim::defs::DefId);
                let steps: Vec<String> = parts
                    .iter()
                    .enumerate()
                    .map(|(i, s)| match i {
                        0 => format!("{} {}", s.label, s.delta),
                        _ => format!("{} {:+}", s.label, s.delta),
                    })
                    .collect();
                let row = lua.create_table()?;
                row.set("value", value)?;
                row.set("why", format!("{} {value} = {}", d.label, steps.join(", ")))?;
                t.set(d.id.as_str(), row)?;
            }
            Ok(Some(t))
        });
        view!("stances", (), |lua, l, _a| {
            let t = lua.create_table()?;
            let defs = &l.world.defs;
            let mut order: Vec<usize> = (0..defs.stances.len()).collect();
            order.sort_by_key(|&s| (defs.stances[s].order, s));
            for s in order {
                let d = &defs.stances[s];
                let row = lua.create_table()?;
                row.set("id", d.id.as_str())?;
                row.set("label", d.label.as_str())?;
                row.set("icon", d.icon.as_str())?;
                row.set("active", l.world.stance == Some(s as rim_sim::defs::DefId))?;
                t.push(row)?;
            }
            Ok(t)
        });
        view!("items", (), |lua, l, _a| {
            let t = lua.create_table()?;
            for d in l.world.defs.things.iter().filter(|d| d.category == rim_sim::defs::Category::Item) {
                let row = lua.create_table()?;
                row.set("id", d.id.as_str())?;
                row.set("label", d.label.as_str())?;
                row.set("color", format!("#{:02x}{:02x}{:02x}", d.rgb[0], d.rgb[1], d.rgb[2]))?;
                t.push(row)?;
            }
            Ok(t)
        });
        view!("zones", (), |lua, l, _a| {
            let t = lua.create_table()?;
            let zones = &l.world.zones;
            for z in &zones.list {
                let row = lua.create_table()?;
                row.set("id", z.id)?;
                row.set("name", z.name.as_str())?;
                row.set("cells", zones.cells.iter().filter(|&&c| c == z.id).count())?;
                let allows = lua.create_table()?;
                for &d in &z.allows {
                    allows.set(l.world.defs.thing(d).id.as_str(), true)?;
                }
                row.set("allows", allows)?;
                t.push(row)?;
            }
            Ok(t)
        });
        // Everything the Work Board paints, in one read: columns in
        // tie-break order with their demand, and a row per colonist.
        view!("board", (), |lua, l, _a| {
            let w = l.world;
            let defs = &w.defs;
            let levels = defs.priority_scale.levels;
            // "High" is the better half of the scale: 1-2 of 4, 1-4 of 9.
            let high = (levels / 2).max(1);
            let colonists: Vec<Entity> = w.colonists().collect();
            let waiting = rim_sim::ai::work_waiting(w);
            let t = lua.create_table()?;
            t.set("levels", levels)?;
            t.set("high", high)?;
            let cols = lua.create_table()?;
            let rows = lua.create_table()?;
            let mut on = vec![(0u32, 0u32); defs.work_types.len()];
            for &e in &colonists {
                let Ok(p) = w.ecs.get::<&Pawn>(e) else { continue };
                let row = lua.create_table()?;
                row.set("id", e.to_bits().get())?;
                row.set("name", p.name.as_str())?;
                row.set("job", rim_sim::order::job_text(w, &p))?;
                let cells = lua.create_table()?;
                for &wt in &defs.work_order {
                    let (value, parts) = rim_sim::rules::explain(defs, &w.rules, &p, wt);
                    let d = &defs.work_types[wt as usize];
                    let skill = d.skill_r.map(|k| p.skill(k));
                    let cell = lua.create_table()?;
                    cell.set("base", p.priority(defs, wt))?;
                    cell.set("value", value)?;
                    let steps: Vec<String> = parts
                        .iter()
                        .enumerate()
                        .map(|(i, s)| {
                            if i == 0 {
                                format!("{} {}", s.label, s.delta)
                            } else {
                                format!("{} {:+}", s.label, s.delta)
                            }
                        })
                        .collect();
                    cell.set("why", format!("{} {value} = {}", d.label, steps.join(", ")))?;
                    if let Some(s) = skill {
                        cell.set("skill", s)?;
                        cell.set("skill_frac", s as f64 / rim_sim::world::SKILL_MAX as f64)?;
                    }
                    cells.push(cell)?;
                    let o = &mut on[wt as usize];
                    o.0 += (value > 0) as u32;
                    o.1 += (value > 0 && value <= high) as u32;
                }
                row.set("cells", cells)?;
                rows.push(row)?;
            }
            for &wt in &defs.work_order {
                let d = &defs.work_types[wt as usize];
                let col = lua.create_table()?;
                col.set("id", d.id.as_str())?;
                col.set("label", d.label.as_str())?;
                col.set("icon", d.icon.as_str())?;
                col.set("skill", d.skill_r.map(|k| defs.skills[k as usize].label.clone()))?;
                col.set("waiting", waiting[wt as usize])?;
                col.set("on", on[wt as usize].0)?;
                col.set("high", on[wt as usize].1)?;
                cols.push(col)?;
            }
            t.set("cols", cols)?;
            t.set("rows", rows)?;
            Ok(t)
        });
        view!("priorities", u64, |lua, l, id| {
            let Some(p) = Entity::from_bits(id).and_then(|e| l.world.ecs.get::<&Pawn>(e).ok()) else {
                return Ok(None);
            };
            let t = lua.create_table()?;
            let defs = &l.world.defs;
            for (w, d) in defs.work_types.iter().enumerate() {
                t.set(d.id.as_str(), p.priority(defs, w as rim_sim::defs::DefId))?;
            }
            Ok(Some(t))
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
        view!("saves", (), |lua, l, _a| {
            let t = lua.create_table()?;
            for s in &l.client.saves {
                let row = lua.create_table()?;
                row.set("path", s.path.as_str())?;
                row.set("file", s.file.as_str())?;
                row.set("day", s.day)?;
                row.set("colonists", lua.create_sequence_from(s.colonists.iter().map(String::as_str))?)?;
                row.set("age", s.age)?;
                row.set("error", s.error.as_deref())?;
                t.push(row)?;
            }
            Ok(t)
        });
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
        // Every bound action with the key it currently has, for the palette.
        let r = self.reg.clone();
        view.set(
            "binds",
            lua.create_function(move |lua, ()| {
                let reg = r.borrow();
                let t = lua.create_table()?;
                let mut seen: Vec<&str> = Vec::new();
                for b in reg.binds.iter().rev() {
                    if seen.contains(&b.id.as_str()) {
                        continue;
                    }
                    seen.push(&b.id);
                    let row = lua.create_table_with_capacity(0, 4)?;
                    row.raw_set("id", b.id.as_str())?;
                    row.raw_set("label", b.label.as_str())?;
                    row.raw_set("key", reg.key_overrides.get(&b.id).unwrap_or(&b.key).as_str())?;
                    row.raw_set("owner", &*b.owner)?;
                    t.raw_push(row)?;
                }
                // Declaration order reads better than reverse.
                let n = t.raw_len();
                let out = lua.create_table_with_capacity(n, 0)?;
                for i in (1..=n).rev() {
                    out.raw_push(t.raw_get::<Value>(i)?)?;
                }
                Ok(out)
            })?,
        )?;
        Ok(view)
    }

    fn report_conflicts(&mut self) {
        let reg = self.reg.borrow();
        let mut seen_ids: Vec<&str> = Vec::new();
        for b in &reg.binds {
            if seen_ids.contains(&b.id.as_str()) {
                continue;
            }
            seen_ids.push(&b.id);
            let owners: Vec<&str> = reg.binds.iter().filter(|o| o.id == b.id).map(|o| &*o.owner).collect();
            if owners.iter().any(|o| *o != owners[0]) {
                self.warnings.push(format!(
                    "UI conflict: action '{}' bound by {} ('{}' wins by load order)",
                    b.id,
                    owners.iter().map(|o| format!("'{o}'")).collect::<Vec<_>>().join(" and "),
                    owners.last().unwrap()
                ));
            }
        }
        // One key, two actions: the later declaration wins.
        let mut by_key: Vec<(&str, &str)> = Vec::new();
        for b in reg.binds.iter().filter(|b| seen_ids.contains(&b.id.as_str())) {
            let key = reg.key_overrides.get(&b.id).map(String::as_str).unwrap_or(&b.key);
            if key.is_empty() {
                continue;
            }
            if let Some((_, other)) = by_key.iter().find(|(k, id)| *k == key && *id != b.id) {
                self.warnings
                    .push(format!("UI conflict: key '{key}' bound by '{other}' and '{}' ('{}' wins)", b.id, b.id));
            }
            by_key.retain(|(_, id)| *id != b.id);
            by_key.push((key, &b.id));
        }
        let mut seen: Vec<&str> = Vec::new();
        for w in &reg.windows {
            if seen.contains(&w.id.as_str()) {
                continue;
            }
            seen.push(&w.id);
            let owners: Vec<&str> = reg.windows.iter().filter(|o| o.id == w.id).map(|o| &*o.owner).collect();
            if owners.iter().any(|o| *o != owners[0]) {
                self.warnings.push(format!(
                    "UI conflict: window '{}' declared by {} ('{}' wins by load order)",
                    w.id,
                    owners.iter().map(|o| format!("'{o}'")).collect::<Vec<_>>().join(" and "),
                    owners.last().unwrap()
                ));
            }
        }
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

    /// Declared windows, one per id: the last declaration wins.
    pub fn windows(&self) -> Vec<WindowDecl> {
        let reg = self.reg.borrow();
        let mut out: Vec<WindowDecl> = Vec::new();
        for w in &reg.windows {
            match out.iter_mut().find(|o| o.id == w.id) {
                Some(o) => *o = w.clone(),
                None => out.push(w.clone()),
            }
        }
        out
    }

    /// The action bound to a key, by its current key (overrides applied).
    pub fn bind_for_key(&self, key: &str) -> Option<Function> {
        if key.is_empty() {
            return None;
        }
        let reg = self.reg.borrow();
        let mut seen: Vec<&str> = Vec::new();
        for b in reg.binds.iter().rev() {
            if seen.contains(&b.id.as_str()) {
                continue;
            }
            seen.push(&b.id);
            if reg.key_overrides.get(&b.id).map(String::as_str).unwrap_or(&b.key) == key {
                return Some(b.func.clone());
            }
        }
        None
    }

    /// The default key of a bound action.
    pub fn default_key(&self, id: &str) -> Option<String> {
        self.reg.borrow().binds.iter().rev().find(|b| b.id == id).map(|b| b.key.clone())
    }

    pub fn set_key_override(&mut self, id: &str, key: Option<String>) {
        let mut reg = self.reg.borrow_mut();
        match key {
            Some(k) => {
                reg.key_overrides.insert(id.to_string(), normalise_key(&k));
            }
            None => {
                reg.key_overrides.remove(id);
            }
        }
    }

    pub fn key_overrides(&self) -> Vec<(String, String)> {
        let reg = self.reg.borrow();
        let mut v: Vec<(String, String)> = reg.key_overrides.iter().map(|(a, b)| (a.clone(), b.clone())).collect();
        v.sort();
        v
    }

    pub fn take_input_sets(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.reg.borrow_mut().input_sets)
    }

    pub fn take_focus_req(&mut self) -> Option<String> {
        self.reg.borrow_mut().focus_req.take()
    }

    pub fn take_window_ops(&mut self) -> Vec<WindowOp> {
        std::mem::take(&mut self.reg.borrow_mut().window_ops)
    }

    pub fn set_window_open(&mut self, id: &str, open: bool) {
        self.reg.borrow_mut().window_open.insert(id.to_string(), open);
    }

    /// Build the open windows, in the order given: each one's chrome (the
    /// registered `ui.window_chrome` function, given the window's record)
    /// around its component. Without a chrome function the component
    /// stands alone. Sizes are logical.
    pub fn build_windows(
        &mut self,
        open: &[(String, (f32, f32))],
        world: &World,
        client: &ClientView,
        engine: &EngineInfo,
        theme: &Theme,
        lists: &ListEnv,
    ) -> Vec<(String, Node)> {
        let decls = self.windows();
        let chrome = self.reg.borrow().chrome.clone();
        self.run_build(world, client, engine, theme, lists, |b| {
            open.iter()
                .filter_map(|(id, size)| {
                    let decl = decls.iter().find(|d| d.id == *id)?;
                    let key = key_for(1, 0, Some(id));
                    let node = match &chrome {
                        Some((chrome_owner, f)) => {
                            let win = b.vm.lua.create_table().ok()?;
                            let _ = win.set("id", decl.id.as_str());
                            let _ = win.set("title", decl.title.as_str());
                            let _ = win.set("w", size.0);
                            let _ = win.set("h", size.1);
                            let _ = win.set("resizable", decl.resizable);
                            let _ = win.set("comp", decl.comp.as_str());
                            match b.call(chrome_owner, f, win) {
                                Ok(Value::Table(t)) => b.convert(&t, key, chrome_owner, None),
                                Ok(_) => Some(b.fail(chrome_owner, key, "window chrome", "must return a node".into())),
                                Err(e) => Some(b.fail(chrome_owner, key, "window chrome", e)),
                            }
                        }
                        None => b.expand_slot(&decl.comp, key, &decl.owner),
                    }?;
                    Some((id.clone(), node))
                })
                .collect()
        })
    }

    /// Run `f` with a builder: scripts may read the world, their time is
    /// charged to their mod, and errors are kept once each.
    fn run_build<R>(
        &mut self,
        world: &World,
        client: &ClientView,
        engine: &EngineInfo,
        theme: &Theme,
        lists: &ListEnv,
        f: impl FnOnce(&mut Builder) -> R,
    ) -> R {
        let view: Table = self.lua.globals().get("view").unwrap();
        let mut times: HashMap<Rc<str>, f64> = HashMap::new();
        let mut errors = Vec::new();
        let out = self.lend(world, client, engine, || {
            let mut b = Builder { vm: self, theme, view: &view, times: &mut times, errors: &mut errors, lists };
            f(&mut b)
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
        out
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

    /// Build the given mounted components. Returns (mount, tree) pairs;
    /// a mount whose component was removed has none.
    pub fn build(
        &mut self,
        mounts: Vec<Mount>,
        world: &World,
        client: &ClientView,
        engine: &EngineInfo,
        theme: &Theme,
        lists: &ListEnv,
    ) -> Vec<(Mount, Node)> {
        let out = self.run_build(world, client, engine, theme, lists, |b| {
            mounts
                .into_iter()
                .filter_map(|m| {
                    let key = key_for(0, 0, Some(&m.id));
                    let owner = m.owner.clone();
                    b.expand_slot(&m.id, key, &owner).map(|n| (m, n))
                })
                .collect::<Vec<_>>()
        });
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
                    if !seen.contains(id) && !reg.comps.contains_key(id) && !reg.declared_ids.contains(id) {
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

    /// Run a handler with arguments and hand back what it returned.
    pub fn call_with(
        &mut self,
        f: &Function,
        args: impl mlua::IntoLuaMulti,
        world: &World,
        client: &ClientView,
        engine: &EngineInfo,
    ) -> Option<Value> {
        self.deadline.set(Some(Instant::now() + CALL_DEADLINE));
        let r = self.lend(world, client, engine, || f.call::<Value>(args));
        self.deadline.set(None);
        match r {
            Ok(v) => Some(v),
            Err(e) => {
                let e = format!("handler: {}", first_line(&e.to_string()));
                if !self.errors.contains(&e) {
                    self.errors.push(e);
                }
                None
            }
        }
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

    /// Every key `ui.t` has been asked for so far, sorted.
    pub fn string_keys(&self) -> Vec<String> {
        let mut v: Vec<String> = self.reg.borrow().used_strings.iter().cloned().collect();
        v.sort();
        v
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

/// The string literals a script assigns to `id`: `id = "core:inspector.thing"`.
fn declared_ids(src: &str) -> Vec<String> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = src[from..].find("id").map(|i| i + from) {
        from = i + 2;
        if i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_') {
            continue;
        }
        let rest = src[from..].trim_start();
        let Some(rest) = rest.strip_prefix('=').filter(|r| !r.starts_with('=')) else { continue };
        let rest = rest.trim_start();
        let Some(q) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') else { continue };
        if let Some(end) = rest[1..].find(q) {
            out.push(rest[1..1 + end].to_string());
        }
    }
    out
}

/// What a virtual list needs from last frame to know its visible span:
/// scroll offsets by node key, and rects by id.
pub struct ListEnv<'a> {
    pub scroll: &'a HashMap<u64, f32>,
    pub rects: &'a HashMap<String, crate::layout::Rect>,
    pub keys: &'a HashMap<String, u64>,
    /// The mods' images, for sizing image nodes at build.
    pub images: &'a crate::image::Images,
    /// Text inputs' buffers, by node id.
    pub edits: &'a HashMap<String, crate::edit::EditState>,
}

struct Builder<'a> {
    vm: &'a UiVm,
    theme: &'a Theme,
    view: &'a Table,
    times: &'a mut HashMap<Rc<str>, f64>,
    errors: &'a mut Vec<String>,
    lists: &'a ListEnv<'a>,
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

    /// Overscan: rows built beyond the visible span on each side, so a
    /// scroll of a few pixels needs no rebuild to show what enters.
    const OVERSCAN: usize = 4;

    fn expand_list(&mut self, t: &Table, key: u64, owner: &Rc<str>, id: Option<&str>) -> Node {
        let what = id.unwrap_or("list");
        let Some(id) = id else {
            return self.fail(owner, key, what, "a list needs an id (its scroll position is kept by it)".into());
        };
        let count: usize = match t.get::<Option<f64>>("count") {
            Ok(Some(n)) if n >= 0.0 => n as usize,
            _ => return self.fail(owner, key, what, "a list needs count".into()),
        };
        let row_fn: Function = match t.get::<Option<Function>>("row") {
            Ok(Some(f)) => f,
            _ => return self.fail(owner, key, what, "a list needs row(i)".into()),
        };
        let row_h = match t.get::<Value>("row_h") {
            Ok(v) if !matches!(v, Value::Nil) => match crate::node::size_of(self.theme, "space", "row_h", &v) {
                Ok(h) => h,
                Err(e) => return self.fail(owner, key, what, e),
            },
            _ => return self.fail(owner, key, what, "a list needs row_h (every row is that tall)".into()),
        };
        // The table becomes the scroll node; the list props leave with it.
        let _ = t.raw_set("kind", "scroll");
        for k in ["count", "row", "row_h"] {
            let _ = t.raw_set(k, Value::Nil);
        }
        let ctx = Ctx { theme: self.theme, owner: owner.clone(), images: self.lists.images, edits: self.lists.edits };
        let mut node = match node_from_table(&ctx, t, key) {
            Ok(n) => n,
            Err(e) => return self.fail(owner, key, what, e),
        };
        let seen_h = self.lists.rects.get(id).map(|r| r[3]).unwrap_or(400.0 * self.theme.scale);
        let scrolled = self.lists.keys.get(id).and_then(|k| self.lists.scroll.get(k)).copied().unwrap_or(0.0);
        // The scroll offset is clamped after layout, so a wheel past the end
        // arrives here unclamped: keep at least the last row in the span, or
        // the area's content would be spacers alone and measure as empty.
        let first = ((scrolled / row_h.max(1.0)) as usize).saturating_sub(Self::OVERSCAN).min(count.saturating_sub(1));
        let last = (((scrolled + seen_h) / row_h.max(1.0)) as usize + 1 + Self::OVERSCAN)
            .clamp(first + 1, count.max(1))
            .min(count);
        // Stand-ins for the rows off screen: fixed height, and a floor on
        // it, since a box in a full column would otherwise shrink.
        let spacer = |key: u64, h: f32| Node {
            style: crate::node::Style { h: crate::node::Len::Px(h), min_h: Some(h), ..Default::default() },
            ..crate::node::blank(key, owner.clone())
        };
        node.children.push(spacer(key_for(key, 0, None), first as f32 * row_h));
        for i in first..last {
            let v = match self.call(owner, &row_fn, i as i64 + 1) {
                Ok(v) => v,
                Err(e) => {
                    node.children.push(self.fail(owner, key_for(key, i + 1, None), what, e));
                    continue;
                }
            };
            let Value::Table(rt) = v else { continue };
            let ck = key_for(key, i + 1, None);
            if let Some(mut n) = self.convert(&rt, ck, owner, None) {
                n.style.h = crate::node::Len::Px(row_h);
                node.children.push(n);
            }
        }
        node.children.push(spacer(key_for(key, count + 2, None), (count - last) as f32 * row_h));
        node
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

        // A virtual list is a scroll area that builds only the rows on
        // screen: a spacer stands in for the rows above, another for the
        // rows below, so scrolling and clamping need nothing new.
        if kind.as_deref() == Some("list") {
            return Some(self.expand_list(t, key, owner, id.as_deref()));
        }
        let ctx = Ctx { theme: self.theme, owner: owner.clone(), images: self.lists.images, edits: self.lists.edits };
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
    let t = lua.create_table_with_capacity(0, 14)?;
    t.raw_set("id", e.to_bits().get())?;
    t.raw_set("name", p.name.as_str())?;
    t.raw_set("label", cd.label.as_str())?;
    t.raw_set("faction", p.faction.name())?;
    t.raw_set("player", p.faction == Faction::Player)?;
    t.raw_set("founder", p.founder)?;
    t.raw_set("drafted", p.drafted)?;
    t.raw_set("asleep", p.asleep)?;
    t.raw_set("hp", p.hp.max(0))?;
    t.raw_set("max_hp", cd.max_hp)?;
    t.raw_set("health", (p.hp.max(0) as f64 / cd.max_hp as f64).clamp(0.0, 1.0))?;
    t.raw_set("job", rim_sim::order::job_text(w, &p))?;
    t.raw_set("selected", client.selected == Some(e))?;
    let needs = lua.create_table_with_capacity(p.needs.len(), 0)?;
    for &(nid, v) in &p.needs {
        let nd = defs.need(nid);
        let row = lua.create_table_with_capacity(0, 5)?;
        row.raw_set("id", nd.id.as_str())?;
        row.raw_set("label", nd.label.as_str())?;
        row.raw_set("value", v as f64 / NEED_MAX as f64)?;
        row.raw_set("color", format!("#{:02x}{:02x}{:02x}", nd.rgb[0], nd.rgb[1], nd.rgb[2]))?;
        row.raw_set("low", v < (nd.seek_below * NEED_MAX as f64) as i32 && nd.satisfier != Satisfier::Rest)?;
        needs.raw_push(row)?;
    }
    t.raw_set("needs", needs)?;
    // Every skill there is, in def order, with the work types it trains.
    let skills = lua.create_table_with_capacity(defs.skills.len(), 0)?;
    for (k, sd) in defs.skills.iter().enumerate() {
        let k = k as DefId;
        let (level, xp) = (p.skill(k), p.skills.iter().find(|s| s.0 == k).map_or(0, |s| s.1));
        let (lo, hi) = (skill_xp(level), skill_xp((level + 1).min(SKILL_MAX)));
        let trains: Vec<&str> = defs
            .work_order
            .iter()
            .map(|&w| &defs.work_types[w as usize])
            .filter(|w| w.skill_r == Some(k))
            .map(|w| w.label.as_str())
            .collect();
        let row = lua.create_table_with_capacity(0, 5)?;
        row.raw_set("id", sd.id.as_str())?;
        row.raw_set("label", sd.label.as_str())?;
        row.raw_set("level", level)?;
        row.raw_set("progress", if hi > lo { (xp.saturating_sub(lo)) as f64 / (hi - lo) as f64 } else { 1.0 })?;
        row.raw_set("trains", trains.join(", "))?;
        skills.raw_push(row)?;
    }
    t.raw_set("skills", skills)?;
    if let Some(tool) = p.hand.and_then(|h| w.thing(h)) {
        t.raw_set("hand", defs.thing(tool.def).label.as_str())?;
    }
    if let Some(l) = &p.carry {
        let of = l.made_of.map(|m| format!(" ({})", defs.thing(m).label)).unwrap_or_default();
        t.raw_set("carrying", format!("{} ×{}{of}", defs.thing(l.def).label, l.count))?;
    }
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
        if let Ok(r) = w.ecs.get::<&Regrow>(e) {
            // With several harvests, say which: a gathered oak can still be chopped.
            if td.harvest.len() > 1 {
                let named: Vec<String> = r
                    .entries()
                    .filter_map(|(h, _)| td.harvest_by_key(h))
                    .map(|h| w.defs.designations[h.desig_r as usize].label.to_lowercase())
                    .collect();
                s = format!("{s} ({} regrowing)", named.join(", "));
            } else {
                s.push_str(" (regrowing)");
            }
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

/// One thing on the map, for the inspector.
fn thing_table(lua: &Lua, w: &World, e: Entity) -> mlua::Result<Option<Table>> {
    let Some(th) = w.thing(e) else { return Ok(None) };
    let td = w.defs.thing(th.def);
    let t = lua.create_table()?;
    t.set("id", e.to_bits().get())?;
    t.set("def", td.id.as_str())?;
    t.set("label", td.label.as_str())?;
    t.set("count", th.count)?;
    t.set("hp", th.hp)?;
    t.set("max_hp", w.stat(e, "hp").map_or(td.hp as i64, |m| m.round() as i64))?;
    t.set("made_of", w.ecs.get::<&rim_sim::world::MadeOf>(e).ok().map(|m| w.defs.thing(m.0).label.clone()))?;
    t.set("blueprint", w.ecs.get::<&Blueprint>(e).is_ok())?;
    let designated = w.ecs.get::<&rim_sim::world::Designated>(e).ok().map(|d| d.0);
    t.set("designated", designated.map(|d| w.defs.designations[d as usize].label.clone()))?;
    t.set("why", rim_sim::ai::work_blocked(w, e))?;
    Ok(Some(t))
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
