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
use rim_sim::world::{skill_xp, Blueprint, Faction, Job, MsgKind, Pawn, Regrow, Thing, World, NEED_MAX, SKILL_MAX};
use rim_sim::TICKS_PER_DAY;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Room labels show from this zoom in, in logical pixels a cell. Further
/// out they would crowd the plan (and roofs take over there).
const ROOM_LABEL_ZOOM: f32 = 14.0;
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
    /// What of the hover the tree being built has read (`Hover` bits):
    /// it depends on those (see `UiVm::reads_hover`).
    hover_read: Cell<u8>,
}

struct Lease<'a> {
    world: &'a World,
    client: &'a ClientView,
    engine: &'a EngineInfo,
    hover_read: &'a Cell<u8>,
}

/// What the pointer is over, as bits: which part of it a tree read, or
/// which part changed.
pub struct Hover;

impl Hover {
    /// The cell (`view.hover`).
    pub const CELL: u8 = 1;
    /// The pawn (who is `hovered` among `view.visible_pawns`).
    pub const PAWN: u8 = 2;
}

fn lease(l: &Lent) -> mlua::Result<Lease<'_>> {
    let (w, c, e) = (l.world.get(), l.client.get(), l.engine.get());
    if w.is_null() || c.is_null() || e.is_null() {
        return Err(mlua::Error::runtime("view is only available while the UI is building or handling input"));
    }
    // SAFETY: set only for the duration of `UiVm::build`/`call_handler`,
    // which hold shared borrows of all three for that whole time.
    Ok(unsafe { Lease { world: &*w, client: &*c, engine: &*e, hover_read: &l.hover_read } })
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
    /// A side panel's reserved height, in logical pixels: the panel's top
    /// stays at the top of it, so content that changes height grows down
    /// into the room instead of moving the header.
    pub slot: Option<f32>,
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
    /// A sheet: a screen the engine places in the band between the docked
    /// columns, one open at a time. It is not moved or resized by hand.
    pub sheet: bool,
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
    /// What a context menu request calls (`ui.on_context`): core's menus.
    context: Option<(Rc<str>, Function)>,
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

/// Below this many logical pixels wide, `view.compact()` is true.
pub const COMPACT_BELOW: f32 = 1440.0;
/// The player's UI scale stays in this range.
pub const UI_SCALE: (f32, f32) = (0.75, 2.0);

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
    /// Mounts (by key) and windows (by "window:" and id) whose last build
    /// read the hover, and which part (`Hover` bits). Only these rebuild
    /// when the pointer moves to another cell or pawn.
    hover_readers: HashMap<String, u8>,
    seen_ids: RefCell<HashSet<String>>,
    /// The player's UI scale, for `view.ui_scale`; set by the engine.
    pub(crate) ui_scale: Rc<Cell<f32>>,
    /// Item looks handed to scripts by index (`view.look`, token.rs).
    looks: Rc<RefCell<crate::token::Looks>>,
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
            hover_read: Cell::new(0),
        });
        let mut vm = UiVm {
            lua,
            reg,
            lent,
            deadline,
            warnings: Vec::new(),
            errors: Vec::new(),
            mod_time: HashMap::new(),
            hover_readers: HashMap::new(),
            seen_ids: RefCell::new(HashSet::new()),
            unknown_checked: false,
            ui_scale: Rc::new(Cell::new(1.0)),
            looks: Rc::new(RefCell::new(crate::token::Looks::default())),
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
                const LAYERS: &[&str] = &[
                    "top", "bottom", "left", "right", "float", "anchored", "cursor", "popup", "modal", "windows",
                    "title",
                ];
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
                let slot = opts.as_ref().and_then(|o| o.get::<Option<f32>>("slot").ok().flatten());
                if let Some(s) = slot {
                    if !(s > 0.0 && s.is_finite()) {
                        return Err(rt(format!("slot must be a positive height, got {s}")));
                    }
                }
                let mut reg = r.borrow_mut();
                let owner: Rc<str> = reg.current.as_str().into();
                reg.mounts.push(Mount { layer, id, order, align, owner, refresh, slot });
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
                    sheet: opts.get::<Option<bool>>("sheet").ok().flatten().unwrap_or(false),
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
        // ui.sheet(): the open sheet's id, or nil. The last declaration of
        // an id decides whether it is a sheet, as everywhere.
        let r = self.reg.clone();
        ui.set(
            "sheet",
            lua.create_function(move |_, ()| {
                let reg = r.borrow();
                let mut seen: Vec<&str> = Vec::new();
                for w in reg.windows.iter().rev() {
                    if seen.contains(&w.id.as_str()) {
                        continue;
                    }
                    seen.push(&w.id);
                    if w.sheet && reg.window_open.get(&w.id).copied().unwrap_or(false) {
                        return Ok(Some(w.id.clone()));
                    }
                }
                Ok(None)
            })?,
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
        // ui.on_context(fn(subject, x, y)): what a right-click on a menu
        // subject calls. Core's menus module sets it; one handler, the last
        // set wins.
        let r = self.reg.clone();
        ui.set(
            "on_context",
            lua.create_function(move |_, f: Function| {
                let mut reg = r.borrow_mut();
                let owner: Rc<str> = reg.current.as_str().into();
                reg.context = Some((owner, f));
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
        act!("select", (Option<u64>, Option<bool>), |(id, add)| match (id.and_then(Entity::from_bits), add) {
            (Some(e), Some(true)) => UiAction::ToggleSelect(e),
            (e, _) => UiAction::Select(e),
        });
        act!("focus", u64, |id| match Entity::from_bits(id) {
            Some(e) => UiAction::Focus(e),
            None => return Err(rt("bad entity id")),
        });
        act!("tool", String, |key| UiAction::Tool(key));
        act!("stuff", String, |id| UiAction::Stuff(id));
        act!("speed", u32, |s| UiAction::Speed(s));
        act!("toggle_pause", (), |_a| UiAction::TogglePause);
        act!("undo", (), |_a| UiAction::Undo);
        act!("turn", (), |_a| UiAction::Turn);
        act!("order", (String, i32, i32, Option<u64>), |(key, x, y, on)| UiAction::Order {
            key,
            cell: rim_sim::IVec::new(x, y),
            on: on.and_then(Entity::from_bits),
        });
        act!("draft", (u64, bool), |(id, on)| match Entity::from_bits(id) {
            Some(e) => UiAction::Draft(e, on),
            None => return Err(rt("bad entity id")),
        });
        act!("set_priority", (u64, String, u8), |(id, work, level)| match Entity::from_bits(id) {
            Some(e) => UiAction::SetPriority(e, work, level),
            None => return Err(rt("bad entity id")),
        });
        act!("clear_priority", (u64, String), |(id, work)| match Entity::from_bits(id) {
            Some(e) => UiAction::ClearPriority(e, work),
            None => return Err(rt("bad entity id")),
        });
        // Roles are 1-based in Luau, as board.roles lists them.
        let role = |r: u16| r.checked_sub(1).ok_or_else(|| rt("roles count from 1"));
        act!("assign_role", (u64, u16), |(id, r)| match Entity::from_bits(id) {
            Some(e) => UiAction::AssignWorkRole(e, role(r)?),
            None => return Err(rt("bad entity id")),
        });
        act!("set_role_priority", (u16, String, Option<u8>), |(r, work, level)| UiAction::SetRolePriority(
            role(r)?,
            work,
            level
        ));
        act!("role_from_colonist", (String, u64), |(label, id)| match Entity::from_bits(id) {
            Some(e) => UiAction::CreateRoleFromPawn(label, e),
            None => return Err(rt("bad entity id")),
        });
        act!("role_from_role", (String, u16), |(label, r)| UiAction::CreateRoleFromRole(label, role(r)?));
        act!("delete_role", u16, |r| UiAction::DeleteRole(role(r)?));
        act!("set_rule_enabled", (String, bool), |(id, on)| UiAction::SetRuleEnabled(id, on));
        act!("mark_urgent", (u64, bool), |(id, on)| match Entity::from_bits(id) {
            Some(e) => UiAction::MarkUrgent(e, on),
            None => return Err(rt("bad entity id")),
        });
        act!("set_stance", String, |id| UiAction::SetStance(id));
        act!("zone_allow", (u32, String, bool), |(zone, item, on)| UiAction::ZoneAllow(zone, item, on));
        act!("store_level", (Value, u8), |(store, level)| UiAction::StoreLevel(store_ref(&store)?, level));
        act!("store_filter", (Value, Table), |(store, edit)| UiAction::StoreFilter(
            store_ref(&store)?,
            filter_edit(&edit)?
        ));
        act!("select_zone", Option<u32>, |zone| UiAction::SelectZone(zone));
        act!("cycle_overlay", (), |_a| UiAction::CycleOverlay);
        act!("set_overlay", Option<usize>, |i| UiAction::SetOverlay(i.map(|i| i.saturating_sub(1))));
        act!("toggle_profiler", (), |_a| UiAction::ToggleProfiler);
        act!("toggle_devtools", (), |_a| UiAction::ToggleDevtools);
        act!("toggle_outlines", (), |_a| UiAction::ToggleOutlines);
        act!("preview", Option<String>, |key| UiAction::Preview(key));
        act!("zoom", f32, |f| match f.is_finite() && f > 0.0 {
            true => UiAction::Zoom(f.clamp(0.25, 4.0)),
            false => return Err(rt("act.zoom: wants a positive factor")),
        });
        act!("scroll_mode", String, |m| match m.as_str() {
            "auto" | "zoom" | "pan" => UiAction::ScrollMode(m),
            _ => return Err(rt("act.scroll_mode: auto, zoom or pan")),
        });
        act!("ui_scale", f32, |s| match s.is_finite() {
            true => UiAction::UiScale(s.clamp(UI_SCALE.0, UI_SCALE.1)),
            false => return Err(rt("act.ui_scale: wants a number from 0.75 to 2")),
        });
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
        view!("selected_zone", (), |_lua, l, _a| Ok(l.client.selected_zone));
        view!("store", Value, |lua, l, store| store_table(lua, l.world, store_ref(&store)?));
        view!("item_categories", (), |lua, l, _a| categories_table(lua, l.world));
        // Every selected id: the group when several are, else the one.
        view!("selection", (), |lua, l, _a| {
            let t = lua.create_table()?;
            if l.client.group.is_empty() {
                if let Some(e) = l.client.selected {
                    t.raw_push(e.to_bits().get())?;
                }
            } else {
                for e in &l.client.group {
                    t.raw_push(e.to_bits().get())?;
                }
            }
            Ok(t)
        });
        view!("shift", (), |_lua, l, _a| Ok(l.client.shift));
        view!("show_profiler", (), |_lua, l, _a| Ok(l.client.show_profiler));
        view!("show_devtools", (), |_lua, l, _a| Ok(l.client.show_devtools));
        view!("hint", (), |_lua, l, _a| Ok(l.client.hint.clone()));
        view!("screen", (), |_lua, l, _a| {
            let s = l.client.scale.max(0.01);
            Ok((l.client.screen.0 / s, l.client.screen.1 / s))
        });
        // A small screen, in logical pixels (a big UI scale makes any
        // screen small): core's components pick denser layouts.
        view!("compact", (), |_lua, l, _a| Ok(l.client.screen.0 / l.client.scale.max(0.01) < COMPACT_BELOW));
        let scale = self.ui_scale.clone();
        view.set("ui_scale", lua.create_function(move |_, ()| Ok(scale.get()))?)?;
        // A thing's look, for an item token: made once per thing and
        // material, and handed out by index (token.rs).
        let (lent, looks) = (self.lent.clone(), self.looks.clone());
        view.set(
            "look",
            lua.create_function(move |_, (thing, made_of): (mlua::LuaString, Option<mlua::LuaString>)| {
                let l = lease(&lent)?;
                let made_of = made_of.as_ref().map(|m| m.to_str()).transpose()?;
                looks.borrow_mut().id(&l.world.defs, &thing.to_str()?, made_of.as_deref().unwrap_or("")).map_err(rt)
            })?,
        )?;

        view!("colonists", Option<usize>, |lua, l, max| {
            let t = lua.create_table()?;
            for e in l.world.colonists().take(max.unwrap_or(usize::MAX)) {
                if let Some(p) = pawn_table(lua, l.world, l.client, e)? {
                    t.raw_push(p)?;
                }
            }
            Ok(t)
        });
        // The colonists as the people column needs them: no needs or skills,
        // so a colony of two hundred costs a fraction of `view.colonists`.
        view!("people", (), |lua, l, _a| {
            let t = lua.create_table()?;
            for e in l.world.colonists() {
                let Ok(p) = l.world.ecs.get::<&Pawn>(e) else { continue };
                if !p.active || p.dead {
                    continue;
                }
                let cd = l.world.defs.creature(p.def);
                let row = lua.create_table_with_capacity(0, 9)?;
                row.raw_set("id", e.to_bits().get())?;
                row.raw_set("name", p.name.as_str())?;
                row.raw_set("label", cd.label.as_str())?;
                row.raw_set("drafted", p.drafted)?;
                row.raw_set("asleep", p.asleep)?;
                row.raw_set("health", (p.hp.max(0) as f64 / cd.max_hp as f64).clamp(0.0, 1.0))?;
                row.raw_set("job", rim_sim::order::job_text(l.world, &p))?;
                row.raw_set("idle", matches!(p.job, Job::Idle | Job::Wander { .. }))?;
                row.raw_set("selected", l.client.is_selected(e))?;
                t.raw_push(row)?;
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
            l.hover_read.set(l.hover_read.get() | Hover::PAWN);
            let t = lua.create_table()?;
            let (sw, sh) = l.client.screen;
            for &e in &l.world.pawns {
                let Ok(p) = l.world.ecs.get::<&Pawn>(e) else { continue };
                if !p.active || p.dead || p.pos.z != l.client.level {
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
                row.raw_set("selected", l.client.is_selected(e))?;
                row.raw_set("hovered", l.client.hover_pawn == Some(e))?;
                // Logical pixels, like every size a component writes.
                row.raw_set("radius", cd.size * l.client.cam.2 / l.client.scale.max(0.1))?;
                t.raw_push(row)?;
            }
            Ok(t)
        });
        // Walled rooms on screen, for their labels (DESIGN.md §6c). Each
        // label goes on the free cell nearest the middle of what's on
        // screen of the room, so it stays in view as the camera moves.
        view!("visible_rooms", (), |lua, l, _a| {
            let t = lua.create_table()?;
            let (w, cv) = (l.world, l.client);
            let (cx, cy, z) = cv.cam;
            if z / cv.scale.max(0.1) < ROOM_LABEL_ZOOM {
                return Ok(t);
            }
            let (sw, sh) = cv.screen;
            let x0 = ((cx - sw / 2.0 / z).floor() as i32).max(0);
            let y0 = ((cy - sh / 2.0 / z).floor() as i32).max(0);
            let x1 = ((cx + sw / 2.0 / z).ceil() as i32).min(w.map.w - 1);
            let y1 = ((cy + sh / 2.0 / z).ceil() as i32).min(w.map.h - 1);
            // Room id -> (sum x, sum y, cells on screen), then the nearest cell.
            let mut seen: std::collections::BTreeMap<u32, (i64, i64, i64)> = Default::default();
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let p = rim_sim::IVec::new(x, y);
                    let Some(r) = w.map.room_at(p).filter(|r| !r.touches_edge) else { continue };
                    let e = seen.entry(r.id).or_default();
                    *e = (e.0 + x as i64, e.1 + y as i64, e.2 + 1);
                }
            }
            let mut best: std::collections::BTreeMap<u32, (f32, rim_sim::IVec)> = Default::default();
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let p = rim_sim::IVec::new(x, y);
                    let Some(r) = w.map.room_at(p).filter(|r| !r.touches_edge) else { continue };
                    let (sx, sy, n) = seen[&r.id];
                    let (mx, my) = (sx as f32 / n as f32, sy as f32 / n as f32);
                    // Furniture is in the way of a label; bare floor isn't.
                    let busy = if w.map.fixture_at(p).is_some() { 4.0 } else { 0.0 };
                    let d = (x as f32 - mx).powi(2) + (y as f32 - my).powi(2) + busy;
                    let b = best.entry(r.id).or_insert((f32::MAX, p));
                    if d < b.0 {
                        *b = (d, p);
                    }
                }
            }
            let temp = w.defs.lookup("field", "temperature");
            for (id, (_, p)) in best {
                let r = w.map.room_by_id(id);
                // A sliver of a room at the screen's edge gets no label.
                if seen[&id].2 < 4 && r.cells >= 4 {
                    continue;
                }
                let row = lua.create_table_with_capacity(0, 8)?;
                row.raw_set("id", id)?;
                row.raw_set("x", p.x)?;
                row.raw_set("y", p.y)?;
                row.raw_set("cells", r.cells)?;
                row.raw_set("open", !r.enclosed())?;
                if let Some(d) = w.room_role(p) {
                    let role = &w.defs.room_roles[d as usize];
                    row.raw_set("role", role.label.as_str())?;
                    row.raw_set("role_id", role.id.as_str())?;
                }
                if let Some(fi) = temp {
                    let fd = &w.defs.fields[fi as usize];
                    let v = w.fields.value(&w.defs, &w.map, fi as usize, p);
                    row.raw_set("temperature", format!("{v:.0}{}", fd.unit))?;
                }
                t.raw_push(row)?;
            }
            Ok(t)
        });
        view!("speech", (), |lua, l, _a| {
            let t = lua.create_table()?;
            let now = l.world.tick;
            for s in &l.world.speech {
                let age = now.saturating_sub(s.tick);
                if age >= s.ticks as u64 {
                    continue;
                }
                let row = lua.create_table_with_capacity(0, 4)?;
                row.raw_set("id", s.pawn.to_bits().get())?;
                row.raw_set("text", s.text.as_str())?;
                row.raw_set("age", age as f64 / s.ticks as f64)?;
                row.raw_set("priority", s.priority)?;
                t.raw_push(row)?;
            }
            Ok(t)
        });
        // Newest first; `skip` pages back through the log, so a list of it
        // fetches only the lines in view.
        view!("messages", (usize, Option<usize>), |lua, l, (max, skip)| {
            let t = lua.create_table()?;
            for m in l.world.messages.iter().rev().skip(skip.unwrap_or(0)).take(max) {
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
                row.set("day", m.tick / TICKS_PER_DAY + 1)?;
                let h = rim_sim::world::hour_at(m.tick);
                row.set("clock", format!("{:02}:{:02}", h as u32, (h.fract() * 60.0) as u32))?;
                t.push(row)?;
            }
            Ok(t)
        });
        view!("message_count", (), |_lua, l, _a| Ok(l.world.messages.len()));
        // The last order and its age in seconds, or nil.
        view!("last_order", (), |lua, l, _a| {
            let Some((label, age)) = &l.client.last_order else { return Ok(None) };
            let t = lua.create_table()?;
            t.set("label", label.as_str())?;
            t.set("age", *age)?;
            Ok(Some(t))
        });
        // Every order the selected colonists could be given at a spot, for
        // the orders menu: merged by key, with how many of them it's on
        // offer to. Walks the map (reachability), so call it once per
        // opening, not per frame.
        view!("orders", (i32, i32, Option<u64>), |lua, l, (x, y, on)| {
            let w = l.world;
            let cell = rim_sim::IVec::new(x, y);
            let on = on.and_then(Entity::from_bits);
            let actors: Vec<Entity> = if l.client.group.is_empty() {
                l.client.selected.into_iter().collect()
            } else {
                l.client.group.clone()
            }
            .into_iter()
            .filter(|&e| w.ecs.get::<&Pawn>(e).is_ok_and(|p| p.faction == Faction::Player && p.active && !p.dead))
            .collect();
            // (key, label, damaging, how many can, a reason if one can't).
            let mut merged: Vec<(String, String, bool, usize, Option<String>)> = Vec::new();
            for &a in &actors {
                for c in rim_sim::order::options(w, a, cell, on) {
                    let can = usize::from(c.order.is_some());
                    match merged.iter_mut().find(|m| m.0 == c.key) {
                        Some(m) => {
                            m.3 += can;
                            if m.4.is_none() {
                                m.4 = c.reason;
                            }
                        }
                        None => merged.push((c.key, c.label, c.damaging, can, c.reason)),
                    }
                }
            }
            let rows = lua.create_table()?;
            for (key, label, damaging, can, reason) in merged {
                let r = lua.create_table()?;
                r.set("key", key)?;
                r.set("label", label)?;
                r.set("group", if damaging { "damaging" } else { "do" })?;
                if can == 0 {
                    r.set("disabled", reason.unwrap_or_else(|| "can't now".into()))?;
                } else if can < actors.len() {
                    r.set("trailing", format!("{can} of {}", actors.len()))?;
                }
                rows.raw_push(r)?;
            }
            let caption = on
                .and_then(|e| w.ecs.get::<&Pawn>(e).ok().map(|p| w.defs.creature(p.def).label.clone()))
                .or_else(|| {
                    [w.map.fixture_at(cell), w.map.item_at(cell), w.map.floor_at(cell)]
                        .into_iter()
                        .flatten()
                        .find_map(|e| w.thing(e).map(|t| w.defs.thing(t.def).label.clone()))
                })
                .or_else(|| {
                    w.map.inb(cell).then(|| w.defs.terrain[w.map.terrain[w.map.idx(cell)] as usize].label.clone())
                });
            let t = lua.create_table()?;
            t.set("rows", rows)?;
            t.set("caption", caption)?;
            t.set("actors", actors.len())?;
            if let [one] = actors.as_slice() {
                t.set("actor", w.ecs.get::<&Pawn>(*one).map(|p| p.name.clone()).ok())?;
            }
            Ok(t)
        });
        // How many things each designation has marked, by designation id:
        // one pass over the marked things, for the Orders tray.
        view!("marked", (), |lua, l, _a| {
            let mut counts = vec![0u32; l.world.defs.designations.len()];
            for d in l.world.ecs.query::<&rim_sim::world::Designated>().iter() {
                if let Some(c) = counts.get_mut(d.0 as usize) {
                    *c += 1;
                }
            }
            let t = lua.create_table()?;
            for (i, n) in counts.into_iter().enumerate().filter(|(_, n)| *n > 0) {
                t.set(l.world.defs.designations[i].id.as_str(), n)?;
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
        view!("overlay", (), |_lua, l, _a| {
            Ok(match l.client.storage_overlay {
                true => Some("Storage".to_string()),
                false => l.client.overlay.map(|i| l.world.defs.fields[i].label.clone()),
            })
        });
        view!("stock", (), |lua, l, _a| stock_table(lua, l.world));
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
                let (value, parts) = rim_sim::rules::explain(l.world, &p, w as rim_sim::defs::DefId);
                let row = lua.create_table()?;
                row.set("value", value)?;
                row.set("why", why_text(defs, &d.label, value, &parts))?;
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
        // Standing orders (DESIGN.md §4d): the rules on colony readings, with
        // their reading now, their marks, what they do and whether they act.
        view!("standing", (), |lua, l, _a| {
            let w = l.world;
            let defs = &w.defs;
            let t = lua.create_table()?;
            for (i, rd) in defs.priority_rules.iter().enumerate() {
                let (Some(reading), Some(band)) = (&rd.when.reading, rd.when.band_r) else { continue };
                let row = lua.create_table()?;
                row.set("id", rd.id.as_str())?;
                row.set("label", rim_sim::rules::label(defs, i as u16))?;
                row.set("reading", reading.as_str())?;
                row.set("value", w.standing.reading(reading))?;
                row.set("band", band_text(band))?;
                row.set("effect", rule_effect(defs, rd))?;
                row.set("season", (!rd.when.season.is_empty()).then(|| rd.when.season.join(", ")))?;
                row.set("crossed", w.standing.on.contains(&rd.id))?;
                row.set("enabled", !w.standing.off.contains(&rd.id))?;
                row.set("acting", w.rules.on.contains(&(i as u16)))?;
                t.push(row)?;
            }
            Ok(t)
        });
        // Urgent marks (DESIGN.md §4d): the job on a tile a mark could go on.
        view!("markable", (i32, i32), |lua, l, (x, y)| {
            let w = l.world;
            let Some(e) = w.markable_at(rim_sim::IVec::new(x, y)) else { return Ok(None) };
            let t = lua.create_table()?;
            t.set("id", e.to_bits().get())?;
            t.set("urgent", w.ecs.get::<&rim_sim::world::Urgent>(e).is_ok())?;
            let label = match (w.thing(e), w.ecs.get::<&Pawn>(e)) {
                (Some(th), _) => w.defs.thing(th.def).label.clone(),
                (None, Ok(p)) => p.name.clone(),
                _ => String::new(),
            };
            t.set("label", label)?;
            Ok(Some(t))
        });
        view!("urgent_count", (), |_lua, l, _a| Ok(l.world.urgent_count()));
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
        view!("store_levels", (), |lua, l, _a| {
            lua.create_sequence_from(l.world.defs.store_priority.labels.iter().map(String::as_str))
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
                for &d in &z.filter.allows {
                    allows.set(l.world.defs.thing(d).id.as_str(), true)?;
                }
                row.set("allows", allows)?;
                row.set("level", z.level)?;
                let labels = &l.world.defs.store_priority.labels;
                row.set("level_label", labels.get(z.level as usize).map_or("", String::as_str))?;
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
            let scale = &defs.priority_scale;
            t.set("labels", (1..=levels).map(|l| scale.name(l)).collect::<Vec<_>>())?;
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
                if let Some(r) = w.work_role_of(&p) {
                    row.set("role", r as u32 + 1)?;
                    row.set("role_label", w.work_roles[r as usize].label.as_str())?;
                }
                let cells = lua.create_table()?;
                for &wt in &defs.work_order {
                    let (value, parts) = rim_sim::rules::explain(w, &p, wt);
                    let d = &defs.work_types[wt as usize];
                    let skill = d.skill_r.map(|k| p.skill(k));
                    let cell = lua.create_table()?;
                    cell.set("base", w.base_priority(&p, wt))?;
                    cell.set("value", value)?;
                    cell.set("inherit", w.inherited_priority(&p, wt))?;
                    cell.set("pinned", p.own_priority(wt).is_some())?;
                    // A ring on the board: a planner chose it and may change it.
                    cell.set("planned", p.own_priority(wt).is_none() && w.is_planned(&p) && p.planned(wt).is_some())?;
                    cell.set("why", why_text(defs, &d.label, value, &parts))?;
                    // Auto's reason on its own, for the plan view.
                    let reason =
                        parts.iter().find(|p| p.kind == rim_sim::rules::PartKind::Plan).map(|p| p.label.as_str());
                    cell.set("reason", reason)?;
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
                col.set("default", d.priority.min(levels))?;
                col.set("on", on[wt as usize].0)?;
                col.set("high", on[wt as usize].1)?;
                cols.push(col)?;
            }
            t.set("cols", cols)?;
            t.set("rows", rows)?;
            let roles = lua.create_table()?;
            let default = w.default_work_role().map(|r| r as usize);
            for (i, r) in w.work_roles.iter().enumerate() {
                let role = lua.create_table()?;
                role.set("index", i + 1)?;
                role.set("id", r.def.clone())?;
                role.set("label", r.label.as_str())?;
                role.set("order", r.order)?;
                role.set("edited", r.edited)?;
                role.set("planned", r.planner.is_some())?;
                role.set("default", default == Some(i))?;
                let set = lua.create_table()?;
                for &(wt, l) in &r.priorities {
                    set.set(defs.work_types[wt as usize].id.as_str(), l)?;
                }
                role.set("levels", set)?;
                roles.push(role)?;
            }
            t.set("roles", roles)?;
            Ok(t)
        });
        // The why panel: each work type, and why the colonist takes it or not.
        view!("explain_work", u64, |lua, l, id| {
            let Some(e) = Entity::from_bits(id) else { return Ok(None) };
            let t = lua.create_table()?;
            for x in rim_sim::ai::explain_work(l.world, e) {
                let row = lua.create_table()?;
                row.set("work", l.world.defs.work_types[x.work as usize].id.as_str())?;
                row.set("level", x.level)?;
                row.set("why", rim_sim::order::work_why_text(l.world, &x))?;
                row.set("picked", matches!(x.why, rim_sim::ai::Why::Picked(_)))?;
                row.set("dist", x.dist)?;
                row.set("urgent", x.urgent)?;
                t.push(row)?;
            }
            Ok(Some(t))
        });
        view!("priorities", u64, |lua, l, id| {
            let Some(p) = Entity::from_bits(id).and_then(|e| l.world.ecs.get::<&Pawn>(e).ok()) else {
                return Ok(None);
            };
            let t = lua.create_table()?;
            let defs = &l.world.defs;
            for (w, d) in defs.work_types.iter().enumerate() {
                t.set(d.id.as_str(), l.world.base_priority(&p, w as rim_sim::defs::DefId))?;
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
                row.set("category", tool.category.as_str())?;
                row.set("group", tool.group.as_str())?;
                row.set("cost", tool.cost.as_str())?;
                row.set("work", tool.work)?;
                row.set("hp", tool.hp)?;
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
        view!("hover", (), |lua, l, _a| {
            l.hover_read.set(l.hover_read.get() | Hover::CELL);
            hover_table(lua, l.world, l.client)
        });
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
    /// A sheet's size as declared, or None if `id` isn't a sheet. The last
    /// declaration wins, as in `windows`.
    pub fn sheet_size(&self, id: &str) -> Option<(f32, f32)> {
        self.reg.borrow().windows.iter().rev().find(|w| w.id == id).filter(|w| w.sheet).map(|w| (w.w, w.h))
    }

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

    /// Ask for the context menu of a subject at a point in logical pixels,
    /// through the handler `ui.on_context` set. Returns whether one did.
    pub fn context(
        &mut self,
        kind: &str,
        id: &str,
        at: (f32, f32),
        world: &World,
        client: &ClientView,
        engine: &EngineInfo,
    ) -> bool {
        let Some((_, f)) = self.reg.borrow().context.clone() else { return false };
        let subject = match self.lua.create_table() {
            Ok(t) => t,
            Err(_) => return false,
        };
        let _ = subject.set("kind", kind);
        // Entity ids are integers: pass them as integers, not strings.
        match id.parse::<i64>() {
            Ok(n) => {
                let _ = subject.set("id", n);
            }
            Err(_) => {
                let _ = subject.set("id", id);
            }
        }
        self.call_with(&f, (subject, at.0, at.1), world, client, engine);
        true
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
        let built: Vec<(String, Node, u8)> = self.run_build(world, client, engine, theme, lists, |b| {
            open.iter()
                .filter_map(|(id, size)| {
                    let decl = decls.iter().find(|d| d.id == *id)?;
                    let key = key_for(1, 0, Some(id));
                    b.vm.lent.hover_read.set(0);
                    let node = match &chrome {
                        Some((chrome_owner, f)) => {
                            let win = b.vm.lua.create_table().ok()?;
                            let _ = win.set("id", decl.id.as_str());
                            let _ = win.set("title", decl.title.as_str());
                            let _ = win.set("w", size.0);
                            let _ = win.set("h", size.1);
                            let _ = win.set("resizable", decl.resizable && !decl.sheet);
                            let _ = win.set("sheet", decl.sheet);
                            let _ = win.set("comp", decl.comp.as_str());
                            match b.call(chrome_owner, f, win) {
                                Ok(Value::Table(t)) => b.convert(&t, key, chrome_owner, None),
                                Ok(_) => Some(b.fail(chrome_owner, key, "window chrome", "must return a node".into())),
                                Err(e) => Some(b.fail(chrome_owner, key, "window chrome", e)),
                            }
                        }
                        None => b.expand_slot(&decl.comp, key, &decl.owner),
                    }?;
                    Some((id.clone(), node, b.vm.lent.hover_read.get()))
                })
                .collect()
        });
        self.hover_readers.retain(|k, _| !k.starts_with("window:"));
        built
            .into_iter()
            .map(|(id, node, read)| {
                self.note_hover(format!("window:{id}"), read);
                (id, node)
            })
            .collect()
    }

    fn note_hover(&mut self, key: String, read: u8) {
        if read != 0 {
            self.hover_readers.insert(key, read);
        } else {
            self.hover_readers.remove(&key);
        }
    }

    /// Did this mount's last build read a part of the hover that `moved`
    /// (`Hover` bits)?
    pub fn reads_hover(&self, mount_key: &str, moved: u8) -> bool {
        self.hover_readers.get(mount_key).is_some_and(|r| r & moved != 0)
    }

    /// Did any open window's last build read a part that `moved`?
    pub fn windows_read_hover(&self, moved: u8) -> bool {
        self.hover_readers.iter().any(|(k, r)| k.starts_with("window:") && r & moved != 0)
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
        self.looks.borrow_mut().new_build();
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
        let built = self.run_build(world, client, engine, theme, lists, |b| {
            mounts
                .into_iter()
                .map(|m| {
                    let key = key_for(0, 0, Some(&m.id));
                    let owner = m.owner.clone();
                    b.vm.lent.hover_read.set(0);
                    let n = b.expand_slot(&m.id, key, &owner);
                    (m, n, b.vm.lent.hover_read.get())
                })
                .collect::<Vec<_>>()
        });
        let mut out = Vec::with_capacity(built.len());
        for (m, n, read) in built {
            // A mount that shows nothing until something is hovered still
            // read the hover: it has to rebuild when the hover comes.
            self.note_hover(m.key(), read);
            out.extend(n.map(|n| (m, n)));
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
        let ctx = Ctx {
            theme: self.theme,
            owner: owner.clone(),
            images: self.lists.images,
            edits: self.lists.edits,
            looks: &self.vm.looks,
        };
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
        let ctx = Ctx {
            theme: self.theme,
            owner: owner.clone(),
            images: self.lists.images,
            edits: self.lists.edits,
            looks: &self.vm.looks,
        };
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
    t.raw_set("selected", client.is_selected(e))?;
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

/// Who's on the work at `target`, or who'd take it: "Being done by Bo",
/// "Next: Bo in ~20 s, then Cyd", led by "Urgent · " when the player
/// marked it. At 1x the sim runs 60 ticks a second.
fn takes_text(w: &World, target: Entity) -> String {
    let lead = if w.ecs.get::<&rim_sim::world::Urgent>(target).is_ok() { "Urgent · " } else { "" };
    format!("{lead}{}", taker_text(w, target))
}

fn taker_text(w: &World, target: Entity) -> String {
    let name = |e: Entity| w.ecs.get::<&Pawn>(e).map(|p| p.name.clone()).unwrap_or_default();
    if let Some(&holder) = w.reservations.get(&target) {
        return format!("Being done by {}", name(holder));
    }
    let line = rim_sim::ai::who_takes(w, target);
    let Some(&(first, ticks)) = line.first() else { return "Nobody free would take it now".into() };
    let mut s = format!("Next: {} in ~{} s", name(first), (ticks / 60).max(1));
    if let Some(&(second, _)) = line.get(1) {
        s = format!("{s}, then {}", name(second));
    }
    s
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
            // Walled in, but too wide for its walls: say what would fix it.
            Some(r) if !r.touches_edge => format!("open to the sky: {} cells beyond the roof's reach", r.uncovered),
            Some(_) => "outdoors".into(),
            None => String::new(),
        },
    )?;
    let readings = lua.create_table()?;
    let values = lua.create_table()?;
    // Only fields that vary over the map; the weather readout covers the rest.
    for (fi, fd) in w.defs.fields.iter().enumerate().filter(|(_, fd)| fd.overlay) {
        let value = format!("{:.0}{}", w.fields.value(&w.defs, &w.map, fi, tp), fd.unit);
        readings.push(format!("{} {value}", fd.label))?;
        let row = lua.create_table()?;
        row.set("label", fd.label.as_str())?;
        row.set("value", value)?;
        values.push(row)?;
    }
    t.set("readings", readings)?;
    t.set("values", values)?;
    // Who'd take the work waiting here, only for something with work on it.
    let work_on = [w.map.fixture[i], w.map.floor[i]].into_iter().flatten().find(|&e| {
        w.ecs.get::<&rim_sim::world::Designated>(e).is_ok()
            || w.ecs.get::<&Blueprint>(e).is_ok()
            || w.ecs.get::<&rim_sim::world::Order>(e).is_ok()
    });
    if let Some(e) = work_on {
        t.set("takes", takes_text(w, e))?;
    }
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
    // The store under the cursor, as view.store takes it: a container
    // standing here, else the stockpile the cell is in.
    let container = w.map.fixture[i].filter(|&e| w.ecs.get::<&rim_sim::world::Store>(e).is_ok());
    let store = match (container, w.zones.at(&w.map, tp)) {
        (Some(e), _) => Some(("thing", e.to_bits().get())),
        (None, Some(z)) => Some(("zone", z.id as u64)),
        _ => None,
    };
    if let Some((k, id)) = store {
        let s = lua.create_table()?;
        s.set(k, id)?;
        t.set("store", s)?;
    }
    Ok(Value::Table(t))
}

/// What the colony has, one row per thing it has any of, from the stock
/// ledger: units stored and loose, how many stores hold it, and its
/// category for grouping (DESIGN.md §4f). Rows are in def order.
fn stock_table(lua: &Lua, w: &World) -> mlua::Result<Table> {
    use rim_sim::defs::DefId;
    let defs = &w.defs;
    // How many stores hold each thing: stockpiles by their cells, containers
    // by their slots, each store counted once.
    let mut stores: Vec<std::collections::BTreeSet<(u8, u64)>> = vec![Default::default(); defs.things.len()];
    for (z, c) in w.zones.members() {
        if let Some(t) = w.map.item_at(w.map.pos(c as usize)).and_then(|e| w.thing(e)) {
            stores[t.def as usize].insert((0, z.id as u64));
        }
    }
    for (e, st) in w.ecs.query::<(Entity, &rim_sim::world::Store)>().iter() {
        for x in st.slots.iter().flatten() {
            if let Ok(t) = w.ecs.get::<&Thing>(*x) {
                stores[t.def as usize].insert((1, e.to_bits().get()));
            }
        }
    }
    let out = lua.create_table()?;
    for d in 0..defs.things.len() as DefId {
        let total = w.stock.on_map(d);
        if total == 0 {
            continue;
        }
        let td = defs.thing(d);
        let stored = w.stock.stored(d);
        let r = lua.create_table()?;
        r.set("thing", td.id.as_str())?;
        r.set("label", td.label.as_str())?;
        r.set("total", total)?;
        r.set("stored", stored)?;
        r.set("loose", total.saturating_sub(stored))?;
        r.set("stores", stores[d as usize].len())?;
        r.set("value", td.market_value * total as f64)?;
        let cat = defs.item_categories.iter().find(|c| c.items.contains(&d));
        r.set("category", cat.map(|c| c.id.clone()))?;
        r.set("category_label", cat.map(|c| c.label.clone()))?;
        r.set("category_order", cat.map_or(i32::MAX, |c| c.order))?;
        out.push(r)?;
    }
    Ok(out)
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
    t.set("store", w.ecs.get::<&rim_sim::world::Store>(e).is_ok())?;
    Ok(Some(t))
}

/// A store a script names: a stockpile's id, or `{ zone = id }` or
/// `{ thing = id }` (a container).
fn store_ref(v: &Value) -> mlua::Result<rim_sim::zone::StoreRef> {
    use rim_sim::zone::StoreRef;
    let bad = || rt("a store is a stockpile's id, { zone = id } or { thing = id }");
    match v {
        Value::Integer(n) => Ok(StoreRef::Zone(*n as u32)),
        Value::Number(n) => Ok(StoreRef::Zone(*n as u32)),
        Value::Table(t) => {
            if let Some(z) = t.get::<Option<u32>>("zone")? {
                return Ok(StoreRef::Zone(z));
            }
            match t.get::<Option<u64>>("thing")?.and_then(Entity::from_bits) {
                Some(e) => Ok(StoreRef::Thing(e)),
                None => Err(bad()),
            }
        }
        _ => Err(bad()),
    }
}

/// A filter edit a script sends: `{ thing | category | material = id, on
/// = bool }`, `{ min = n, max = n }` (condition, percent) or `{ all =
/// bool }`.
fn filter_edit(t: &Table) -> mlua::Result<crate::view::UiFilterEdit> {
    use crate::view::UiFilterEdit as E;
    let on = t.get::<Option<bool>>("on")?.unwrap_or(true);
    if let Some(x) = t.get::<Option<String>>("thing")? {
        return Ok(E::Thing(x, on));
    }
    if let Some(x) = t.get::<Option<String>>("category")? {
        return Ok(E::Category(x, on));
    }
    if let Some(x) = t.get::<Option<String>>("material")? {
        return Ok(E::Material(x, on));
    }
    if let Some(all) = t.get::<Option<bool>>("all")? {
        return Ok(E::All(all));
    }
    let (min, max) = (t.get::<Option<u8>>("min")?, t.get::<Option<u8>>("max")?);
    if min.is_some() || max.is_some() {
        return Ok(E::Condition(min.unwrap_or(0), max.unwrap_or(100)));
    }
    Err(rt("a filter edit is { thing | category | material = id, on = bool }, { min, max } or { all = bool }"))
}

/// Everything the store inspector paints, in one read: what a store is,
/// how full, what it holds (a container's slots in order; a stockpile's
/// totals by thing and material) and what it takes.
fn store_table(lua: &Lua, w: &World, store: rim_sim::zone::StoreRef) -> mlua::Result<Option<Table>> {
    use rim_sim::world::{Job, Pawn, Store};
    use rim_sim::zone::StoreRef;
    let defs = &w.defs;
    let t = lua.create_table()?;
    let first_category = |d: rim_sim::defs::DefId| {
        defs.item_categories.iter().position(|c| c.items.contains(&d)).map(|i| &defs.item_categories[i])
    };
    let row = |d: rim_sim::defs::DefId, made_of: Option<rim_sim::defs::DefId>, count: u32, hp: i32, limit: u32| {
        let td = defs.thing(d);
        let r = lua.create_table()?;
        r.set("thing", td.id.as_str())?;
        r.set("label", td.label.as_str())?;
        r.set("made_of", made_of.map(|m| defs.thing(m).id.clone()))?;
        r.set("made_of_label", made_of.map(|m| defs.thing(m).label.clone()))?;
        r.set("count", count)?;
        r.set("limit", limit)?;
        r.set("hp", (hp.max(0) as f64 / defs.full_hp(d, made_of) as f64).clamp(0.0, 1.0))?;
        r.set("value", td.market_value * count as f64)?;
        let cat = first_category(d);
        r.set("category", cat.map(|c| c.id.clone()))?;
        r.set("category_label", cat.map(|c| c.label.clone()))?;
        r.set("category_order", cat.map_or(i32::MAX, |c| c.order))?;
        Ok::<_, mlua::Error>(r)
    };
    let (filter, level, can): (rim_sim::filter::Filter, u8, Vec<rim_sim::defs::DefId>) = match store {
        StoreRef::Zone(z) => {
            let Some(zone) = w.zones.get(z) else { return Ok(None) };
            t.set("kind", "zone")?;
            t.set("id", z)?;
            t.set("name", zone.name.as_str())?;
            // Totals by thing and material, over its cells.
            let mut totals: std::collections::BTreeMap<
                (rim_sim::defs::DefId, Option<rim_sim::defs::DefId>),
                (u32, u32, i32),
            > = std::collections::BTreeMap::new();
            let (mut cells, mut used) = (0u32, 0u32);
            for (_, c) in w.zones.members().filter(|(zn, _)| zn.id == z) {
                cells += 1;
                let Some(e) = w.map.item_at(w.map.pos(c as usize)) else { continue };
                let Some(x) = w.thing(e) else { continue };
                used += 1;
                let e = totals.entry((x.def, w.made_of(e))).or_insert((0, 0, i32::MAX));
                e.0 += x.count;
                e.1 += 1;
                e.2 = e.2.min(x.hp);
            }
            let contents = lua.create_table()?;
            for ((d, m), (count, stacks, hp)) in totals {
                let r = row(d, m, count, hp, defs.thing(d).stack_limit * stacks)?;
                r.set("stacks", stacks)?;
                contents.push(r)?;
            }
            t.set("contents", contents)?;
            t.set("capacity", cells)?;
            t.set("used", used)?;
            let can = (0..defs.things.len() as rim_sim::defs::DefId)
                .filter(|&d| defs.thing(d).category == rim_sim::defs::Category::Item)
                .collect();
            (zone.filter.clone(), zone.level, can)
        }
        StoreRef::Thing(e) => {
            let (Some(th), Ok(st)) = (w.thing(e), w.ecs.get::<&Store>(e).map(|s| (*s).clone())) else {
                return Ok(None);
            };
            let td = defs.thing(th.def);
            let Some(sd) = td.store.as_ref() else { return Ok(None) };
            t.set("kind", "thing")?;
            t.set("id", e.to_bits().get())?;
            t.set("name", td.label.as_str())?;
            let contents = lua.create_table()?;
            let mut used = 0u32;
            for slot in &st.slots {
                match slot.and_then(|x| w.thing(x).map(|t| (x, t))) {
                    Some((x, xt)) => {
                        used += 1;
                        let limit = defs.thing(xt.def).stack_limit.saturating_mul(sd.stack_scale);
                        contents.push(row(xt.def, w.made_of(x), xt.count, xt.hp, limit)?)?;
                    }
                    None => {
                        let r = lua.create_table()?;
                        r.set("empty", true)?;
                        contents.push(r)?;
                    }
                }
            }
            t.set("contents", contents)?;
            t.set("capacity", st.slots.len())?;
            t.set("used", used)?;
            (st.filter.clone(), st.level, sd.accepts_r.clone())
        }
    };
    let labels = &defs.store_priority.labels;
    t.set("level", level)?;
    t.set("level_label", labels.get(level as usize).map_or("", String::as_str))?;
    t.set("levels", lua.create_sequence_from(labels.iter().map(String::as_str))?)?;
    // Haulers bound for it: stacks on their way in.
    let incoming = w
        .pawns
        .iter()
        .filter_map(|&p| w.ecs.get::<&Pawn>(p).ok().map(|p| p.job.clone()))
        .filter(|job| match (job, store) {
            (Job::Haul { into: Some(c), .. }, StoreRef::Thing(e)) => *c == e,
            (Job::Haul { to, into: None, .. }, StoreRef::Zone(z)) => {
                w.zones.at(&w.map, *to).is_some_and(|zn| zn.id == z)
            }
            _ => false,
        })
        .count();
    t.set("incoming", incoming)?;
    let set = |ids: &mut dyn Iterator<Item = rim_sim::defs::DefId>| -> mlua::Result<Table> {
        let s = lua.create_table()?;
        for d in ids {
            s.set(defs.thing(d).id.as_str(), true)?;
        }
        Ok(s)
    };
    let f = lua.create_table()?;
    f.set("allows", set(&mut filter.allows.iter().copied())?)?;
    f.set("refuses", set(&mut filter.refuses.iter().copied())?)?;
    f.set("can", set(&mut can.into_iter())?)?;
    f.set("min", filter.hp[0])?;
    f.set("max", filter.hp[1])?;
    t.set("filter", f)?;
    // Materials a thing can be made of, for the filter's chips.
    let mats = lua.create_table()?;
    for (i, td) in defs.things.iter().enumerate() {
        if td.category == rim_sim::defs::Category::Item && td.stuff.is_some() {
            let m = lua.create_table()?;
            m.set("id", td.id.as_str())?;
            m.set("label", td.label.as_str())?;
            m.set("refused", filter.refuses.binary_search(&(i as rim_sim::defs::DefId)).is_ok())?;
            mats.push(m)?;
        }
    }
    t.set("materials", mats)?;
    Ok(Some(t))
}

/// The item category tree: `roots` in order, and each category by id with
/// its label, children (in order), the items directly in it, and every
/// item under it.
fn categories_table(lua: &Lua, w: &World) -> mlua::Result<Table> {
    let defs = &w.defs;
    let cats = &defs.item_categories;
    let ids = |v: &[rim_sim::defs::DefId], f: &dyn Fn(rim_sim::defs::DefId) -> String| {
        lua.create_sequence_from(v.iter().map(|&d| f(d)))
    };
    let t = lua.create_table()?;
    t.set("roots", ids(&defs.category_roots, &|c| cats[c as usize].id.clone())?)?;
    let by = lua.create_table()?;
    for (i, c) in cats.iter().enumerate() {
        let r = lua.create_table()?;
        r.set("id", c.id.as_str())?;
        r.set("label", c.label.as_str())?;
        r.set("children", ids(&c.children, &|k| cats[k as usize].id.clone())?)?;
        r.set("items", ids(&c.items, &|d| defs.thing(d).id.clone())?)?;
        r.set("under", ids(&defs.category_items(i as rim_sim::defs::DefId), &|d| defs.thing(d).id.clone())?)?;
        by.set(c.id.as_str(), r)?;
    }
    t.set("by_id", by)?;
    Ok(t)
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

/// One wording for a priority's explanation, in the scale's names:
/// "Build: First = default Later · Builder Soon · Cyd's setting First ·
/// Siege −2". Defaults, roles and pins say the level they set; rules say
/// how far they moved it.
fn why_text(defs: &rim_sim::defs::DefDb, work: &str, value: u8, parts: &[rim_sim::rules::Part]) -> String {
    use rim_sim::rules::PartKind;
    let scale = &defs.priority_scale;
    let mut at = 0i32;
    let steps: Vec<String> = parts
        .iter()
        .map(|s| {
            at += s.delta;
            let level = scale.name(at.max(0) as u8);
            match s.kind {
                PartKind::Default => format!("default {level}"),
                PartKind::Role => format!("{} {level}", s.label),
                PartKind::Pin => format!("{}'s setting {level}", s.label),
                PartKind::Plan if s.label.is_empty() => format!("planned {level}"),
                PartKind::Plan => format!("planned {level} ({})", s.label),
                PartKind::Rule if s.delta < 0 => format!("{} −{}", s.label, -s.delta),
                PartKind::Rule if s.delta > 0 => format!("{} +{}", s.label, s.delta),
                PartKind::Rule => format!("{} ±0", s.label),
            }
        })
        .collect();
    format!("{work}: {} = {}", scale.name(value), steps.join(" · "))
}

/// A reading rule's marks as a player reads them: "on under 5, off at 8".
fn band_text(band: rim_sim::defs::Band) -> String {
    let n = |m: i64| {
        let v = m as f64 / 1000.0;
        if v.fract() == 0.0 {
            format!("{v:.0}")
        } else {
            format!("{v:.1}")
        }
    };
    match band {
        rim_sim::defs::Band::Below { on, off } if on == off => format!("under {}", n(on)),
        rim_sim::defs::Band::Below { on, off } => format!("on under {}, off at {}", n(on), n(off)),
        rim_sim::defs::Band::Above { on, off } if on == off => format!("over {}", n(on)),
        rim_sim::defs::Band::Above { on, off } => format!("on over {}, off at {}", n(on), n(off)),
    }
}

/// What a rule does, in words: "Harvest and Hunt one level sooner".
fn rule_effect(defs: &rim_sim::defs::DefDb, rd: &rim_sim::defs::PriorityRuleDef) -> String {
    let names = |ws: Vec<&str>| match ws.as_slice() {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    };
    let label = |w: rim_sim::defs::DefId| defs.work_types[w as usize].label.as_str();
    let mut parts = Vec::new();
    let mut deltas: Vec<i32> = rd.shift_r.iter().map(|s| s.1).collect();
    deltas.sort_unstable();
    deltas.dedup();
    for d in deltas {
        let ws: Vec<&str> = rd.shift_r.iter().filter(|s| s.1 == d).map(|s| label(s.0)).collect();
        let how = match d {
            -1 => "one level sooner".to_string(),
            1 => "one level later".to_string(),
            d if d < 0 => format!("{} levels sooner", -d),
            d => format!("{d} levels later"),
        };
        parts.push(format!("{} {how}", names(ws)));
    }
    for &(w, l) in &rd.set_r {
        parts.push(format!("{} at {}", label(w), defs.priority_scale.name(l)));
    }
    parts.join("; ")
}
