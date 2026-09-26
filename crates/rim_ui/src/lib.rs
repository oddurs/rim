//! rim_ui: the UI engine (DESIGN.md §11).
//!
//! Luau component trees in, a draw list and a glyph atlas out. There is no
//! renderer dependency: layout, text, input routing and the UI VM all run
//! headless, so they're tested in CI without a window.
//!
//! Each frame, `Ui::frame`:
//! 1. routes input against last frame's layout (topmost layer first; panels
//!    swallow clicks; what the UI doesn't handle falls through to the world),
//! 2. builds every mounted component in the UI VM,
//! 3. composes the layers (the docking shell, anchored labels with collision
//!    avoidance, the cursor label, modals, tooltips),
//! 4. lays out (cached by tree hash) and paints.

pub mod api;
pub mod check;
pub mod edit;
pub mod fontcache;
pub mod image;
pub mod layout;
pub mod node;
pub mod paint;
pub mod text;
pub mod theme;
pub mod view;
pub mod vm;

use layout::Rect;
use node::{Anchor, Kind, Len, Node, Style, TextStyle};
use paint::{contains, Draw, Hit, PaintState};
use std::collections::{HashMap, HashSet};
use std::hash::Hasher;
use std::rc::Rc;
use std::time::Instant;
use text::Text;
use theme::{Theme, Token};
use view::{ClientView, UiAction};
use vm::{EngineInfo, InspectInfo, ModDir, UiVm};

/// Raw input for one frame, in physical pixels.
#[derive(Clone, Debug, Default)]
pub struct Input {
    pub mouse: (f32, f32),
    pub left_pressed: bool,
    pub left_released: bool,
    pub right_pressed: bool,
    /// Wheel steps this frame (positive is up).
    pub wheel: f32,
    pub tab: bool,
    pub shift: bool,
    pub enter: bool,
    /// Keys for a focused text input this frame, in order.
    pub keys: Vec<Key>,
    /// Every key pressed this frame by name ("space", "f3", "ctrl+k"), for
    /// bound actions. Ignored while a text input has focus.
    pub pressed: Vec<String>,
    /// Wall-clock seconds.
    pub time: f64,
}

/// A key a text input understands. Tab and Enter are `Input` flags.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Key {
    Char(char),
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Escape,
}

/// What the client needs back from a frame.
#[derive(Default)]
pub struct Output {
    pub draw: Vec<Draw>,
    pub actions: Vec<UiAction>,
    /// The pointer is over UI, so world hover and clicks should be ignored.
    pub mouse_over_ui: bool,
    /// The UI used this frame's left press (don't start a world drag).
    pub captured_left: bool,
    pub captured_right: bool,
    pub captured_wheel: bool,
    /// Tab or Enter went to a focused UI element.
    pub captured_keys: bool,
}

/// A drag across a grid: the grid's key, the value the drag paints, and
/// the cells painted so far so each is reported once.
struct Painting {
    key: u64,
    value: mlua::Value,
    done: HashSet<(usize, usize)>,
}

/// What routing found to run this frame.
enum Call {
    Click(mlua::Function),
    /// An input's text changed, or was submitted.
    Text {
        f: mlua::Function,
        text: String,
    },
    /// The pointer is held on a node with on_drag: fractions across it.
    Drag {
        f: mlua::Function,
        fx: f32,
        fy: f32,
    },
    /// A press on a grid cell: ask the mod what to paint, then paint it.
    Press {
        grid: Rc<node::Grid>,
        key: u64,
        cell: (usize, usize),
    },
    /// The drag entered another cell.
    Paint {
        f: mlua::Function,
        cell: (usize, usize),
        value: mlua::Value,
    },
}

/// One laid-out layer from the last frame, kept for input routing.
struct LayerOut {
    name: &'static str,
    roots: Vec<Node>,
    hits: Vec<Hit>,
    /// Rectangles that stop the pointer reaching anything below.
    solids: Vec<Rect>,
    /// Every node with its rect, for devtools and `find`.
    all: Vec<(Rect, usize, Vec<usize>)>,
    /// The window each root belongs to (windows layer), by root index.
    wins: Vec<Option<String>>,
}

/// Where a window is and whether it shows, in logical pixels so a saved
/// layout reads the same at any scale. Kept in stacking order, last on top.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WindowState {
    #[serde(skip)]
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub open: bool,
}

/// A press on a window's move or resize handle, until release.
struct WinDrag {
    id: String,
    resize: bool,
    start: (f32, f32),
    from: (f32, f32, f32, f32),
}

/// The smallest a window resizes to, logical pixels.
const WIN_MIN: (f32, f32) = (120.0, 80.0);
/// How much of a window must stay on screen when dragged.
const WIN_KEEP: f32 = 80.0;

/// A laid-out tree: its layout hash, the space it had, and its rects.
type CachedLayout = (u64, (f32, f32), Vec<Rect>);

/// Layers bottom to top.
const LAYERS: &[&str] = &["anchored", "docked", "title", "windows", "cursor", "modal", "tooltip"];
const TOOLTIP_DELAY: f64 = 0.45;
/// How many slots away from its anchor a label may move to avoid overlap.
const ANCHOR_SLOTS: usize = 12;
/// How many anchored labels are placed per frame, by priority, inside the
/// viewport. The rest draw nothing.
const ANCHOR_CAP: usize = 48;
/// Trees rebuild on any input or client change, and otherwise at this rate:
/// hover and press restyle at paint time and anchored labels are placed
/// every frame, so between rebuilds nothing visible goes stale.
const REBUILD_EVERY: f64 = 1.0 / 20.0;
/// How often a `refresh = "slow"` mount rebuilds.
const SLOW_EVERY: f64 = 0.25;

/// The client state trees depend on (not the mouse: hover is paint-time).
fn client_hash(c: &ClientView) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    use std::hash::Hash;
    c.selected.map(|e| e.to_bits().get()).hash(&mut h);
    (c.paused, c.speed, c.overlay, c.show_profiler, c.show_devtools).hash(&mut h);
    c.hint.hash(&mut h);
    c.hover_cell.map(|p| (p.x, p.y)).hash(&mut h);
    c.hover_pawn.map(|e| e.to_bits().get()).hash(&mut h);
    for t in &c.tools {
        (&t.key, t.active).hash(&mut h);
    }
    (c.screen.0.to_bits(), c.screen.1.to_bits(), c.scale.to_bits()).hash(&mut h);
    c.title.hash(&mut h);
    for s in &c.saves {
        (&s.path, &s.error).hash(&mut h);
    }
    h.finish()
}

pub struct Ui {
    pub text: Text,
    pub theme: Theme,
    pub vm: UiVm,
    mods: Vec<ModDir>,
    user_scale: f32,
    dpi: f32,
    layers: Vec<LayerOut>,
    hovered: Option<u64>,
    hovered_since: f64,
    pressed: Option<u64>,
    painting: Option<Painting>,
    focused: Option<u64>,
    scroll: HashMap<u64, f32>,
    cache: HashMap<String, CachedLayout>,
    /// One taffy tree, reused for every layout.
    lay: layout::Engine,
    pub info: EngineInfo,
    pub devtools: bool,
    ids: HashMap<String, Rect>,
    /// Node key for each id, to look up per-node state such as scroll offsets.
    id_keys: HashMap<String, u64>,
    reload_stamp: u64,
    reload_checked: f64,
    /// A UI script or theme that failed to reload; the last good UI keeps running.
    pub reload_error: Option<String>,
    pub reloads: u64,
    /// Every built tree from the last frame, by mount id (snapshot tests).
    pub last_trees: Vec<(String, Node)>,
    /// What scripts see of the engine's numbers, refreshed a few times a
    /// second so a timing readout doesn't reflow the UI every frame.
    shown: EngineInfo,
    shown_at: f64,
    /// Natural size and relative rects of small trees (labels, tooltips).
    small: HashMap<u64, ((f32, f32), Vec<Rect>)>,
    built: Vec<(vm::Mount, Node)>,
    /// When each mount was last rebuilt, by `Mount::key`.
    built_at_by: HashMap<String, f64>,
    built_at: f64,
    built_for: u64,
    /// Tree rebuilds so far (tests and the profiler).
    pub builds: u64,
    /// The screen as of the last frame, for placing a window opened from a handler.
    screen: (f32, f32),
    /// Windows the engine manages, in stacking order (last on top).
    windows: Vec<WindowState>,
    win_drag: Option<WinDrag>,
    /// Chrome trees of the open windows from the last rebuild, in stacking order.
    built_wins: Vec<(String, Node)>,
    /// Last frame's window rectangles (physical), in stacking order.
    win_rects: Vec<(String, Rect)>,
    /// A window moved, resized, opened or closed since the layout was last taken.
    layout_dirty: bool,
    /// The mods' PNGs, placed in the atlas as they are drawn.
    pub images: image::Images,
    /// Every text input's buffer, by node id. Outlives rebuilds and reloads.
    edits: HashMap<String, edit::EditState>,
    /// A node a handler asked to focus, resolved once it is laid out.
    focus_pending: Option<String>,
    /// A key override changed since the keybinds were last taken.
    keys_dirty: bool,
}

fn total_scale(dpi: f32, user: f32) -> f32 {
    (dpi * user).max(0.5)
}

fn stamp(mods: &[ModDir]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for m in mods {
        for f in vm::ui_files(&m.dir) {
            h.write(f.to_string_lossy().as_bytes());
            if let Ok(t) = std::fs::metadata(&f).and_then(|md| md.modified()) {
                h.write_u128(t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
            }
        }
    }
    h.finish()
}

impl Ui {
    /// `dpi` is the display's pixel ratio; `user_scale` the player's UI scale.
    pub fn new(mods: Vec<ModDir>, dpi: f32, user_scale: f32) -> Result<Ui, String> {
        let dirs: Vec<(String, &std::path::Path)> = mods.iter().map(|m| (m.id.clone(), m.dir.as_path())).collect();
        let mut theme = Theme::load(&dirs, total_scale(dpi, user_scale));
        let text = Text::new(if theme.font.is_empty() { None } else { Some(theme.font.as_str()) }, &mod_fonts(&dirs))?;
        theme.warnings.extend(text.info.load_errors.iter().map(|e| format!("font file ignored: {e}")));
        let mut images = image::Images::load(&dirs);
        theme.warnings.append(&mut images.warnings);
        if let Some(want) = &text.info.missing {
            let by = theme.set_by.get("font.family").cloned().unwrap_or_default();
            theme.warnings.push(format!(
                "{by}/ui/theme.toml: font '{want}' is not installed; using the system UI font ({})",
                text.info.family
            ));
        }
        let vm = UiVm::load(&mods);
        let info = EngineInfo { font: format!("{} ({})", text.info.family, text.info.source), ..Default::default() };
        let reload_stamp = stamp(&mods);
        Ok(Ui {
            text,
            theme,
            vm,
            mods,
            user_scale,
            dpi,
            layers: Vec::new(),
            hovered: None,
            hovered_since: 0.0,
            pressed: None,
            painting: None,
            focused: None,
            scroll: HashMap::new(),
            cache: HashMap::new(),
            lay: layout::Engine::default(),
            info,
            devtools: false,
            ids: HashMap::new(),
            id_keys: HashMap::new(),
            reload_stamp,
            reload_checked: 0.0,
            reload_error: None,
            reloads: 0,
            last_trees: Vec::new(),
            shown: EngineInfo::default(),
            shown_at: f64::MIN,
            small: HashMap::new(),
            built: Vec::new(),
            built_at_by: HashMap::new(),
            built_at: f64::MIN,
            built_for: 0,
            builds: 0,
            screen: (0.0, 0.0),
            windows: Vec::new(),
            win_drag: None,
            built_wins: Vec::new(),
            win_rects: Vec::new(),
            layout_dirty: false,
            images,
            edits: HashMap::new(),
            focus_pending: None,
            keys_dirty: false,
        })
    }

    /// The player's keys as TOML: only actions rebound from their default.
    pub fn keybinds_toml(&self) -> String {
        let mut keys = toml::Table::new();
        for (id, key) in self.vm.key_overrides() {
            keys.insert(id, toml::Value::String(key));
        }
        let mut doc = toml::Table::new();
        doc.insert("keys".into(), toml::Value::Table(keys));
        toml::to_string(&doc).unwrap_or_default()
    }

    /// Restore keys from `keybinds_toml`. Ids no mod declares are kept for
    /// when their mod comes back.
    pub fn restore_keybinds(&mut self, text: &str) -> Result<(), String> {
        let doc: toml::Table = text.parse().map_err(|e: toml::de::Error| e.message().to_string())?;
        let Some(toml::Value::Table(keys)) = doc.get("keys") else { return Ok(()) };
        for (id, v) in keys {
            let toml::Value::String(k) = v else { return Err(format!("{id}: a key is a string")) };
            self.vm.set_key_override(id, Some(k.clone()));
        }
        Ok(())
    }

    /// Bind an action to another key for this player, or back to its default.
    pub fn rebind(&mut self, id: &str, key: Option<&str>) {
        let default = self.vm.default_key(id);
        let key = key.map(vm::normalise_key).filter(|k| Some(k) != default.as_ref());
        self.vm.set_key_override(id, key);
        self.keys_dirty = true;
    }

    pub fn take_keys_dirty(&mut self) -> bool {
        std::mem::take(&mut self.keys_dirty)
    }

    /// A text input's buffer and caret (tests, devtools).
    pub fn edit_state(&self, id: &str) -> Option<&edit::EditState> {
        self.edits.get(id)
    }

    /// The id of the focused node, if it has one.
    pub fn focused_id(&self) -> Option<String> {
        let f = self.focused?;
        self.id_keys.iter().find(|(_, k)| **k == f).map(|(id, _)| id.clone())
    }

    /// The focused node, if it is a text input: its id and handlers.
    fn focused_input(&self) -> Option<(String, node::InputData)> {
        let f = self.focused?;
        let (layer, h) = self.hit_by_key(f)?;
        let n = self.node_at(layer, h.path[0], &h.path[1..])?;
        let input = n.input.as_ref()?;
        Some((n.id.as_deref()?.to_string(), input.clone()))
    }

    /// Open windows in stacking order, bottom first.
    pub fn window_order(&self) -> Vec<String> {
        self.windows.iter().filter(|w| w.open).map(|w| w.id.clone()).collect()
    }

    /// Where an open window is on screen, physical pixels.
    pub fn window_rect(&self, id: &str) -> Option<Rect> {
        self.win_rects.iter().find(|(w, _)| w == id).map(|(_, r)| *r)
    }

    pub fn is_open(&self, id: &str) -> bool {
        self.windows.iter().any(|w| w.id == id && w.open)
    }

    pub fn open_window(&mut self, id: &str) {
        self.apply_window_op(&vm::WindowOp::Open(id.to_string()));
    }

    pub fn close_window(&mut self, id: &str) {
        self.apply_window_op(&vm::WindowOp::Close(id.to_string()));
    }

    /// The window layout as TOML: one table per window the player has
    /// touched, keyed by id, in logical pixels. Saved beside client
    /// settings, never in a save game.
    pub fn layout_toml(&self) -> String {
        let mut doc = toml::Table::new();
        let mut wins = toml::Table::new();
        for w in &self.windows {
            if let Ok(toml::Value::Table(t)) = toml::Value::try_from(w) {
                wins.insert(w.id.clone(), toml::Value::Table(t));
            }
        }
        doc.insert("window".into(), toml::Value::Table(wins));
        toml::to_string(&doc).unwrap_or_default()
    }

    /// Restore a layout from `layout_toml`. Keyed by id, so a window keeps
    /// its place across a mod update that changes its default size; ids the
    /// mods no longer declare are kept for when they come back.
    pub fn restore_layout(&mut self, text: &str) -> Result<(), String> {
        let doc: toml::Table = text.parse().map_err(|e: toml::de::Error| e.message().to_string())?;
        let Some(toml::Value::Table(wins)) = doc.get("window") else { return Ok(()) };
        for (id, v) in wins {
            let mut w: WindowState =
                v.clone().try_into().map_err(|e: toml::de::Error| format!("{id}: {}", e.message()))?;
            w.id = id.clone();
            w.w = w.w.max(WIN_MIN.0);
            w.h = w.h.max(WIN_MIN.1);
            match self.windows.iter_mut().find(|o| o.id == *id) {
                Some(o) => *o = w,
                None => self.windows.push(w),
            }
        }
        Ok(())
    }

    /// Whether the layout changed since this was last asked, and clears it.
    pub fn take_layout_dirty(&mut self) -> bool {
        std::mem::take(&mut self.layout_dirty)
    }

    fn raise_window(&mut self, id: &str) {
        if let Some(i) = self.windows.iter().position(|w| w.id == id) {
            if i + 1 != self.windows.len() {
                let w = self.windows.remove(i);
                self.windows.push(w);
                self.layout_dirty = true;
            }
        }
    }

    /// A window's record, made from its declaration the first time it is
    /// needed: centred, stepped down and right of any already open.
    fn window_state(&mut self, id: &str, screen: (f32, f32)) -> Option<usize> {
        if let Some(i) = self.windows.iter().position(|w| w.id == id) {
            return Some(i);
        }
        let decl = self.vm.windows().into_iter().find(|d| d.id == id)?;
        let s = self.theme.scale;
        let open = self.windows.iter().filter(|w| w.open).count() as f32;
        let (sw, sh) = (screen.0 / s, screen.1 / s);
        self.windows.push(WindowState {
            id: id.to_string(),
            x: ((sw - decl.w) / 2.0 + open * 24.0).max(0.0),
            y: ((sh - decl.h) / 2.0 + open * 24.0).max(0.0),
            w: decl.w,
            h: decl.h,
            open: false,
        });
        Some(self.windows.len() - 1)
    }

    fn apply_window_op(&mut self, op: &vm::WindowOp) {
        let screen = self.screen;
        let (id, open) = match op {
            vm::WindowOp::Open(id) => (id, Some(true)),
            vm::WindowOp::Close(id) => (id, Some(false)),
            vm::WindowOp::Toggle(id) => (id, None),
        };
        let Some(i) = self.window_state(id, screen) else { return };
        let open = open.unwrap_or(!self.windows[i].open);
        if self.windows[i].open != open {
            self.windows[i].open = open;
            self.layout_dirty = true;
        }
        if open {
            self.raise_window(id);
        }
        self.vm.set_window_open(id, open);
    }

    /// Keep a window's record inside the screen with its title bar reachable.
    fn clamp_window(w: &mut WindowState, screen: (f32, f32), scale: f32) {
        let (sw, sh) = (screen.0 / scale, screen.1 / scale);
        w.w = w.w.max(WIN_MIN.0);
        w.h = w.h.max(WIN_MIN.1);
        w.x = w.x.clamp(WIN_KEEP - w.w, (sw - WIN_KEEP).max(0.0));
        w.y = w.y.clamp(0.0, (sh - 40.0).max(0.0));
    }

    /// Load problems from theme and scripts, for the profiler and mod manager.
    pub fn warnings(&self) -> Vec<String> {
        self.theme.warnings.iter().chain(&self.vm.warnings).chain(&self.vm.errors).cloned().collect()
    }

    pub fn set_dpi(&mut self, dpi: f32) {
        if (dpi - self.dpi).abs() > 1e-3 {
            self.dpi = dpi;
            self.theme.scale = total_scale(dpi, self.user_scale);
            self.cache.clear();
            // Whole-pixel glyph advances read crisper at 1x; at 2x subpixel
            // positions are smoother.
            self.text.set_hinting(dpi < 1.5);
        }
    }

    /// Rectangle of the node with this id in the last frame (autotests, devtools).
    pub fn find(&self, id: &str) -> Option<Rect> {
        self.ids.get(id).copied()
    }

    /// How far the scroll area with this id is scrolled, in pixels.
    pub fn scroll_offset(&self, id: &str) -> Option<f32> {
        let key = self.id_keys.get(id)?;
        Some(self.scroll.get(key).copied().unwrap_or(0.0))
    }

    /// Hot reload: if any UI script or theme changed on disk, load a fresh
    /// VM and theme. On error keep the old ones and report where it broke.
    pub fn check_reload(&mut self, now: f64) -> bool {
        if now - self.reload_checked < 0.5 {
            return false;
        }
        self.reload_checked = now;
        let s = stamp(&self.mods);
        if s == self.reload_stamp {
            return false;
        }
        self.reload_stamp = s;
        self.reload_now()
    }

    /// Reload UI scripts and theme unconditionally.
    pub fn reload_now(&mut self) -> bool {
        let dirs: Vec<(String, &std::path::Path)> = self.mods.iter().map(|m| (m.id.clone(), m.dir.as_path())).collect();
        let mut theme = Theme::load(&dirs, self.theme.scale);
        let mut images = image::Images::load(&dirs);
        theme.warnings.append(&mut images.warnings);
        let vm = UiVm::load(&self.mods);
        let broken: Vec<String> = vm
            .warnings
            .iter()
            .chain(&theme.warnings)
            .filter(|w| !w.starts_with("UI conflict") && !w.starts_with("theme conflict"))
            .cloned()
            .collect();
        if !broken.is_empty() {
            self.reload_error = Some(broken.join("\n"));
            return false;
        }
        // Trees hold references into the old VM (click handlers). Drop them
        // before the VM goes, or releasing them would outlive their Lua.
        self.built.clear();
        self.layers.clear();
        self.last_trees.clear();
        self.theme = theme;
        self.vm = vm;
        self.images = images;
        self.text.forget_images();
        self.cache.clear();
        self.small.clear();
        self.reload_error = None;
        self.reloads += 1;
        true
    }

    /// The hit for a node key, from last frame's layout.
    fn hit_by_key(&self, key: u64) -> Option<(&'static str, Hit)> {
        self.layers.iter().find_map(|l| l.hits.iter().find(|h| h.key == key).map(|h| (l.name, h.clone())))
    }

    fn node_at<'a>(&'a self, layer: &str, root: usize, path: &[usize]) -> Option<&'a Node> {
        let l = self.layers.iter().find(|l| l.name == layer)?;
        let mut n = l.roots.get(root)?;
        for &i in path {
            n = n.children.get(i)?;
        }
        Some(n)
    }

    /// Route input against last frame's layout. Returns clicked handlers.
    fn route(&mut self, input: &Input, out: &mut Output) -> Vec<Call> {
        let (mx, my) = input.mouse;
        let mut handlers: Vec<Call> = Vec::new();
        let mut top: Option<(&'static str, Hit)> = None;
        let mut over_solid = false;
        let mut scroll_hit: Option<Hit> = None;
        for l in self.layers.iter().rev() {
            if top.is_none() {
                for h in l.hits.iter().rev() {
                    let visible = h.clip.is_none_or(|c| contains(c, mx, my));
                    if visible && contains(h.rect, mx, my) {
                        if h.scroll && scroll_hit.is_none() {
                            scroll_hit = Some(h.clone());
                        }
                        if h.interactive {
                            top = Some((l.name, h.clone()));
                            break;
                        }
                    }
                }
            }
            if l.solids.iter().any(|r| contains(*r, mx, my)) {
                over_solid = true;
            }
            if top.is_some() || over_solid {
                break;
            }
        }
        out.mouse_over_ui = top.is_some() || over_solid;
        let hovered = top.as_ref().map(|t| t.1.key);
        if hovered != self.hovered {
            self.hovered = hovered;
            self.hovered_since = input.time;
        }

        if let Some(h) = &scroll_hit {
            if input.wheel != 0.0 {
                let step = 40.0 * self.theme.scale;
                let e = self.scroll.entry(h.key).or_insert(0.0);
                *e = (*e - input.wheel * step).max(0.0);
                out.captured_wheel = true;
            }
        } else if out.mouse_over_ui && input.wheel != 0.0 {
            out.captured_wheel = true;
        }

        // A press on a window brings it to the front, and on its move or
        // resize handle starts a drag the engine follows until release.
        let win_under = match &top {
            Some((layer, h)) if *layer == "windows" => {
                self.layers.iter().find(|l| l.name == "windows").and_then(|l| l.wins.get(h.path[0]).cloned().flatten())
            }
            Some(_) => None,
            None => self.win_rects.iter().rev().find(|(_, r)| contains(*r, mx, my)).map(|(id, _)| id.clone()),
        };
        if input.left_pressed {
            if let Some(id) = &win_under {
                out.mouse_over_ui = true;
                self.raise_window(id);
                let handle =
                    top.as_ref().and_then(|(l, h)| self.node_at(l, h.path[0], &h.path[1..])).and_then(|n| n.handle);
                let resize = match handle {
                    Some(node::Handle::Move) => Some(false),
                    Some(node::Handle::Resize) => Some(true),
                    _ => None,
                };
                if let (Some(resize), Some(w)) = (resize, self.windows.iter().find(|w| w.id == *id)) {
                    self.win_drag =
                        Some(WinDrag { id: id.clone(), resize, start: (mx, my), from: (w.x, w.y, w.w, w.h) });
                }
            }
        }
        if let Some(d) = &self.win_drag {
            let s = self.theme.scale;
            let (dx, dy) = ((mx - d.start.0) / s, (my - d.start.1) / s);
            let screen = self.screen;
            if let Some(w) = self.windows.iter_mut().find(|w| w.id == d.id) {
                if d.resize {
                    (w.w, w.h) = (d.from.2 + dx, d.from.3 + dy);
                } else {
                    (w.x, w.y) = (d.from.0 + dx, d.from.1 + dy);
                }
                Self::clamp_window(w, screen, s);
            }
            out.captured_left = true;
            if input.left_released {
                self.win_drag = None;
                self.layout_dirty = true;
            }
        }
        if input.left_pressed && out.mouse_over_ui {
            out.captured_left = true;
            self.pressed = hovered;
            if let Some((layer, h)) = &top {
                if h.focusable {
                    self.focused = Some(h.key);
                }
                // Clicking an input starts editing what it shows, caret at the end.
                if let Some(n) = self.node_at(layer, h.path[0], &h.path[1..]) {
                    if let (Some(id), Some(inp)) = (n.id.as_deref(), &n.input) {
                        if !self.edits.contains_key(id) {
                            self.edits.insert(id.to_string(), edit::EditState::at_end(&inp.value));
                        }
                    }
                }
                // A grid: the press picks the cell and asks what to paint.
                let grid = self.node_at(layer, h.path[0], &h.path[1..]).and_then(|n| n.grid.clone());
                if let Some(g) = grid {
                    if let Some(cell) = g.cell_at(h.rect, mx, my) {
                        handlers.push(Call::Press { grid: g, key: h.key, cell });
                    }
                }
            }
        }
        // A held pointer on a node with on_drag reports where it is.
        if let Some(p) = self.pressed {
            if let Some((layer, h)) = self.hit_by_key(p) {
                if let Some(f) = self.node_at(layer, h.path[0], &h.path[1..]).and_then(|n| n.on_drag.clone()) {
                    let fx = ((mx - h.rect[0]) / h.rect[2].max(1.0)).clamp(0.0, 1.0);
                    let fy = ((my - h.rect[1]) / h.rect[3].max(1.0)).clamp(0.0, 1.0);
                    handlers.push(Call::Drag { f, fx, fy });
                }
            }
        }
        // A drag across a grid: every newly entered cell gets the value.
        if let Some(Painting { key, value, mut done }) = self.painting.take() {
            if !input.left_released && self.pressed == Some(key) {
                let found = self.hit_by_key(key).and_then(|(layer, h)| {
                    let g = self.node_at(layer, h.path[0], &h.path[1..])?.grid.clone()?;
                    Some((g.cell_at(h.rect, mx, my), g))
                });
                if let Some((Some(cell), g)) = found {
                    if done.insert(cell) {
                        if let Some(f) = g.on_paint.clone() {
                            handlers.push(Call::Paint { f, cell, value: value.clone() });
                        }
                    }
                }
                self.painting = Some(Painting { key, value, done });
            }
        }
        if input.left_released {
            if let (Some(p), Some((layer, h))) = (self.pressed, &top) {
                if p == h.key {
                    let root = h.path[0];
                    if let Some(f) = self.node_at(layer, root, &h.path[1..]).and_then(|n| n.on_click.clone()) {
                        handlers.push(Call::Click(f));
                    }
                    let closes =
                        self.node_at(layer, root, &h.path[1..]).is_some_and(|n| n.handle == Some(node::Handle::Close));
                    if let (true, Some(id)) = (closes, &win_under) {
                        self.close_window(id);
                    }
                }
            }
            if self.pressed.is_some() {
                out.captured_left = true;
            }
            self.pressed = None;
            self.painting = None;
        }
        if input.right_pressed && out.mouse_over_ui {
            out.captured_right = true;
            if let Some((layer, h)) = &top {
                if let Some(f) = self.node_at(layer, h.path[0], &h.path[1..]).and_then(|n| n.on_right_click.clone()) {
                    handlers.push(Call::Click(f));
                }
            }
        }

        // A focused text input takes every key: characters edit the buffer,
        // Enter submits, Escape gives the keyboard back.
        let mut enter = input.enter;
        if let Some((id, inp)) = self.focused_input() {
            out.captured_keys = true;
            let mut changed = false;
            let e = self.edits.entry(id.clone()).or_default();
            for k in &input.keys {
                match k {
                    Key::Char(c) => {
                        e.insert(&c.to_string());
                        changed = true;
                    }
                    Key::Backspace => changed |= e.backspace(),
                    Key::Delete => changed |= e.delete(),
                    Key::Left => e.left(input.shift),
                    Key::Right => e.right(input.shift),
                    Key::Home => e.home(input.shift),
                    Key::End => e.end(input.shift),
                    // Not the buffer's: the input's on_key hears them (a
                    // list under a query moves its selection).
                    Key::Up | Key::Down => {
                        if let Some(f) = &inp.on_key {
                            let name = if *k == Key::Up { "up" } else { "down" };
                            handlers.push(Call::Text { f: f.clone(), text: name.to_string() });
                        }
                    }
                    Key::Escape => self.focused = None,
                }
            }
            let text = e.text.clone();
            if changed {
                if let Some(f) = inp.on_change {
                    handlers.push(Call::Text { f, text: text.clone() });
                }
            }
            if enter {
                if let Some(f) = inp.on_submit {
                    handlers.push(Call::Text { f, text });
                }
                enter = false;
            }
        }
        // Bound actions fire from their keys, unless a text input is typing.
        if !input.pressed.is_empty() && self.focused_input().is_none() {
            for key in &input.pressed {
                if let Some(f) = self.vm.bind_for_key(key) {
                    handlers.push(Call::Click(f));
                    out.captured_keys = true;
                }
            }
        }
        // Keyboard focus: once a UI control has focus (it was clicked or
        // tabbed to), Tab cycles focusable elements and Enter activates. Tab
        // with nothing focused belongs to the game (next colonist).
        if input.tab && self.focused.is_some() {
            let focusables: Vec<(u64, &'static str, Vec<usize>)> = self
                .layers
                .iter()
                .flat_map(|l| l.hits.iter().filter(|h| h.focusable).map(move |h| (h.key, l.name, h.path.clone())))
                .collect();
            if !focusables.is_empty() {
                let i = self.focused.and_then(|f| focusables.iter().position(|x| x.0 == f));
                let n = focusables.len();
                let next = match (i, input.shift) {
                    (Some(i), false) => (i + 1) % n,
                    (Some(i), true) => (i + n - 1) % n,
                    (None, false) => 0,
                    (None, true) => n - 1,
                };
                self.focused = Some(focusables[next].0);
                out.captured_keys = true;
            }
        }
        if enter {
            if let Some(f) = self.focused {
                let found = self
                    .layers
                    .iter()
                    .find_map(|l| l.hits.iter().find(|h| h.key == f).map(|h| (l.name, h.path.clone())));
                if let Some((layer, path)) = found {
                    if let Some(func) = self.node_at(layer, path[0], &path[1..]).and_then(|n| n.on_click.clone()) {
                        handlers.push(Call::Click(func));
                        out.captured_keys = true;
                    }
                }
            }
        }
        handlers
    }

    pub fn frame(&mut self, world: &rim_sim::world::World, client: &ClientView, input: &Input) -> Output {
        let mut out = Output::default();
        self.text.begin_frame();
        if input.time - self.shown_at >= 0.25 {
            self.shown_at = input.time;
            self.shown = EngineInfo { tree: Vec::new(), inspect: None, ..self.info.clone() };
        }
        // Devtools data is live; timing numbers come from the snapshot.
        self.shown.inspect = self.info.inspect.clone();
        self.shown.outlines = self.info.outlines;
        if self.devtools {
            self.shown.tree = self.info.tree.clone();
        } else {
            self.shown.tree.clear();
        }
        let calls = self.route(input, &mut out);
        let handled = !calls.is_empty();
        for call in calls {
            match call {
                Call::Click(f) => self.vm.call_handler(&f, world, client, &self.shown),
                Call::Text { f, text } => {
                    self.vm.call_with(&f, text, world, client, &self.shown);
                }
                Call::Drag { f, fx, fy } => {
                    self.vm.call_with(&f, (fx, fy), world, client, &self.shown);
                }
                Call::Press { grid, key, cell } => {
                    // What the drag paints is the mod's answer to the press;
                    // the pressed cell is painted with it straight away.
                    let (r, c) = (cell.0 as i64 + 1, cell.1 as i64 + 1);
                    let value = match &grid.on_press {
                        Some(f) => self.vm.call_with(f, (r, c), world, client, &self.shown).unwrap_or(mlua::Value::Nil),
                        None => mlua::Value::Nil,
                    };
                    if let Some(f) = &grid.on_paint {
                        self.vm.call_with(f, (r, c, value.clone()), world, client, &self.shown);
                    }
                    self.painting = Some(Painting { key, value, done: HashSet::from([cell]) });
                }
                Call::Paint { f, cell, value } => {
                    self.vm.call_with(&f, (cell.0 as i64 + 1, cell.1 as i64 + 1, value), world, client, &self.shown);
                }
            }
        }
        self.screen = client.screen;
        if let Some(id) = self.vm.take_focus_req() {
            self.focus_pending = Some(id);
        }
        for (id, text) in self.vm.take_input_sets() {
            self.edits.insert(id, edit::EditState::at_end(&text));
        }
        let ops = self.vm.take_window_ops();
        let windows_changed = !ops.is_empty();
        for op in &ops {
            self.apply_window_op(op);
        }
        // A window declared open shows the first time it is seen; a saved
        // layout restored before then has already said where and whether.
        for decl in self.vm.windows() {
            if !self.windows.iter().any(|w| w.id == decl.id) && decl.open {
                self.apply_window_op(&vm::WindowOp::Open(decl.id.clone()));
            }
        }
        let mut actions = self.vm.take_actions();
        if actions.contains(&UiAction::ToggleOutlines) {
            self.info.outlines = !self.info.outlines;
            actions.retain(|a| *a != UiAction::ToggleOutlines);
        }

        let ch = client_hash(client);
        let input_happened = input.left_pressed
            || input.left_released
            || input.right_pressed
            || input.wheel != 0.0
            || !input.keys.is_empty()
            || !input.pressed.is_empty()
            || input.tab
            || input.enter;
        let t0 = Instant::now();
        // Everything rebuilds on input or a client change; otherwise each
        // mount rebuilds at its own cadence, and the windows with the
        // default one.
        let force = handled
            || windows_changed
            || input_happened
            || ch != self.built_for
            || self.built.is_empty()
            || self.devtools;
        let now = input.time;
        let stale = |built_at: f64, every: f64| now - built_at >= every || now < built_at;
        let fast_due = force || stale(self.built_at, REBUILD_EVERY);
        // The title screen is its own layer: before a colony exists nothing
        // else is built, and in a game it isn't.
        let mounts: Vec<vm::Mount> =
            self.vm.mounts().into_iter().filter(|m| (m.layer == "title") == client.title).collect();
        let due: Vec<vm::Mount> = mounts
            .iter()
            .filter(|m| {
                force
                    || match m.refresh {
                        vm::Refresh::Frame => true,
                        vm::Refresh::Fast => {
                            stale(self.built_at_by.get(&m.key()).copied().unwrap_or(f64::MIN), REBUILD_EVERY)
                        }
                        vm::Refresh::Slow => {
                            stale(self.built_at_by.get(&m.key()).copied().unwrap_or(f64::MIN), SLOW_EVERY)
                        }
                    }
            })
            .cloned()
            .collect();
        let rebuilt = !due.is_empty() || fast_due;
        if rebuilt {
            let lists = vm::ListEnv {
                scroll: &self.scroll,
                rects: &self.ids,
                keys: &self.id_keys,
                images: &self.images,
                edits: &self.edits,
            };
            let fresh = self.vm.build(due.clone(), world, client, &self.shown, &self.theme, &lists);
            let mut fresh: HashMap<String, Node> = fresh.into_iter().map(|(m, n)| (m.key(), n)).collect();
            let mut prev: HashMap<String, Node> = self.built.drain(..).map(|(m, n)| (m.key(), n)).collect();
            for m in &due {
                self.built_at_by.insert(m.key(), now);
            }
            self.built = mounts
                .into_iter()
                .filter_map(|m| {
                    let key = m.key();
                    let n = if due.iter().any(|d| d.key() == key) { fresh.remove(&key) } else { prev.remove(&key) }?;
                    Some((m, n))
                })
                .collect();
            if fast_due {
                let open: Vec<(String, (f32, f32))> = self
                    .windows
                    .iter()
                    .filter(|w| w.open && !client.title)
                    .map(|w| (w.id.clone(), (w.w, w.h)))
                    .collect();
                self.built_wins = self.vm.build_windows(&open, world, client, &self.shown, &self.theme, &lists);
                self.built_at = now;
            }
            self.built_for = ch;
            self.builds += 1;
            // Snapshots of what was rebuilt; the rest are as they were.
            for (m, n) in &self.built {
                if !due.iter().any(|d| d.key() == m.key()) {
                    continue;
                }
                match self.last_trees.iter_mut().find(|(id, _)| *id == m.id) {
                    Some(slot) => slot.1 = n.clone(),
                    None => self.last_trees.push((m.id.clone(), n.clone())),
                }
            }
            if fast_due {
                let open: Vec<&str> = self.built_wins.iter().map(|(id, _)| id.as_str()).collect();
                self.last_trees
                    .retain(|(id, _)| self.built.iter().any(|(m, _)| m.id == *id) || open.contains(&id.as_str()));
                for (id, n) in &self.built_wins {
                    match self.last_trees.iter_mut().find(|(i, _)| i == id) {
                        Some(slot) => slot.1 = n.clone(),
                        None => self.last_trees.push((id.clone(), n.clone())),
                    }
                }
            }
        }
        let build_us = t0.elapsed().as_secs_f64() * 1e6;
        let built = std::mem::take(&mut self.built);
        let built_wins = std::mem::take(&mut self.built_wins);

        let (sw, sh) = client.screen;
        let mut layout_us = 0.0;
        let mut paint_us = 0.0;
        let mut layers: Vec<LayerOut> = Vec::new();
        let mut draw = Vec::new();
        let mut ids = HashMap::new();
        let mut id_keys = HashMap::new();
        let mut nodes = 0;
        let mut layouts = self.info.layouts;
        let state_scroll = std::mem::take(&mut self.scroll);
        let state_edits = std::mem::take(&mut self.edits);
        let state = PaintState {
            hovered: self.hovered,
            pressed: self.pressed,
            focused: self.focused,
            scroll: &state_scroll,
            edits: &state_edits,
            disabled_alpha: 0.45,
        };

        for &layer in LAYERS {
            let mut lo = LayerOut {
                name: layer,
                roots: Vec::new(),
                hits: Vec::new(),
                solids: Vec::new(),
                all: Vec::new(),
                wins: Vec::new(),
            };
            // (root node, rects) for this layer.
            let mut placed: Vec<(Node, Vec<Rect>)> = Vec::new();
            let t = Instant::now();
            match layer {
                "docked" => {
                    let shell = self.shell(&built, (sw, sh));
                    let rects = self.layout_cached("docked", &shell, (sw, sh), (0.0, 0.0), &mut layouts);
                    placed.push((shell, rects));
                }
                "anchored" => {
                    for (m, tree) in built.iter().filter(|(m, _)| m.layer == "anchored") {
                        let _ = m;
                        placed.extend(self.place_anchored(tree, world, client));
                    }
                }
                "cursor" => {
                    let off = 16.0 * self.theme.scale;
                    let m = input.mouse;
                    for (_, tree) in built.iter().filter(|(m, _)| m.layer == "cursor") {
                        let rects = self.place_small(tree, (sw * 0.5, sh * 0.5), |(w, h)| {
                            ((m.0 + off).min(sw - w - 4.0).max(0.0), (m.1 + off * 0.75).min(sh - h - 4.0).max(0.0))
                        });
                        placed.push((tree.clone(), rects));
                    }
                }
                "windows" => {
                    let mut y = 60.0 * self.theme.scale;
                    let gap = 12.0 * self.theme.scale;
                    for (_, tree) in built.iter().filter(|(m, _)| m.layer == "windows") {
                        let top = y;
                        let rects = self.place_small(tree, (sw * 0.8, sh * 0.8), |(w, h)| {
                            y = top + h + gap;
                            ((sw - w) / 2.0, top)
                        });
                        placed.push((tree.clone(), rects));
                        lo.wins.push(None);
                    }
                    // Managed windows sit where their records say, in
                    // stacking order; the layout is cached at the origin and
                    // moved, so a drag costs no relayout.
                    let s = self.theme.scale;
                    let screen = (sw, sh);
                    for w in &mut self.windows {
                        Self::clamp_window(w, screen, s);
                    }
                    self.win_rects.clear();
                    for (id, tree) in &built_wins {
                        let Some(w) = self.windows.iter().find(|w| w.id == *id) else { continue };
                        let (x, y, pw, ph) = (w.x * s, w.y * s, w.w * s, w.h * s);
                        let rects: Vec<Rect> = self
                            .layout_cached(&format!("win:{id}"), tree, (pw, ph), (0.0, 0.0), &mut layouts)
                            .into_iter()
                            .map(|r| [r[0] + x, r[1] + y, r[2], r[3]])
                            .collect();
                        self.win_rects.push((id.clone(), [x, y, pw, ph]));
                        placed.push((tree.clone(), rects));
                        lo.wins.push(Some(id.clone()));
                    }
                }
                "title" => {
                    for (_, tree) in built.iter().filter(|(m, _)| m.layer == "title") {
                        let rects = self.place_small(tree, (sw, sh), |(w, h)| ((sw - w) / 2.0, (sh - h) / 2.0));
                        placed.push((tree.clone(), rects));
                    }
                }
                "modal" => {
                    for (_, tree) in built.iter().filter(|(m, _)| m.layer == "modal") {
                        let rects =
                            self.place_small(tree, (sw * 0.8, sh * 0.8), |(w, h)| ((sw - w) / 2.0, (sh - h) / 2.0));
                        placed.push((tree.clone(), rects));
                    }
                    if let Some(err) = &self.reload_error {
                        let n =
                            node::error_node(&self.theme, "rim".into(), 7, "UI reload failed (last good UI kept)", err);
                        let (w, h) = self.lay.natural_size(&n, (sw * 0.8, sh * 0.5), &mut self.text);
                        let rects =
                            self.lay.layout(&n, (w, h), ((sw - w) / 2.0, 40.0 * self.theme.scale), &mut self.text);
                        placed.push((n, rects));
                    }
                }
                "tooltip" => {
                    if let Some(tip) = self.tooltip_text(input.time) {
                        let n = self.tooltip_node(&tip);
                        let off = 14.0 * self.theme.scale;
                        let m = input.mouse;
                        let rects = self.place_small(&n, (sw * 0.4, sh * 0.5), |(w, h)| {
                            ((m.0 + off).min(sw - w - 4.0).max(0.0), (m.1 + off * 1.4).min(sh - h - 4.0).max(0.0))
                        });
                        placed.push((n, rects));
                    }
                }
                _ => {}
            }
            layout_us += t.elapsed().as_secs_f64() * 1e6;

            let t = Instant::now();
            for (ri, (root, rects)) in placed.into_iter().enumerate() {
                let mut hits = Vec::new();
                paint::paint(&root, &rects, (0.0, 0.0), &state, &mut self.text, &self.images, &mut draw, &mut hits);
                for mut h in hits {
                    h.path.insert(0, ri);
                    lo.hits.push(h);
                }
                let solid_layer = matches!(layer, "docked" | "title" | "windows" | "modal");
                let mut i = 0;
                let mut path = vec![ri];
                collect_nodes(&root, &rects, &mut i, &mut path, &mut |n, r, p| {
                    nodes += 1;
                    if solid_layer && (n.style.bg.is_some() || n.is_interactive()) {
                        lo.solids.push(r);
                    }
                    if let Some(id) = &n.id {
                        ids.insert(id.to_string(), r);
                        id_keys.insert(id.to_string(), n.key);
                    }
                    if let Some(aka) = &n.aka {
                        ids.insert(aka.to_string(), r);
                    }
                    lo.all.push((r, 0, p.to_vec()));
                });
                if self.devtools && self.info.outlines && layer != "tooltip" {
                    for (r, _, _) in lo.all.iter().filter(|a| a.2[0] == ri) {
                        draw.push(Draw::Outline { rect: *r, color: [0.35, 0.8, 1.0, 0.5], width: 1.0, radius: 0.0 });
                    }
                }
                lo.roots.push(root);
            }
            paint_us += t.elapsed().as_secs_f64() * 1e6;
            layers.push(lo);
        }
        self.scroll = state_scroll;
        self.edits = state_edits;
        self.clamp_scroll(&layers);

        // Devtools: the node under the cursor, any node at all.
        let mut inspect = None;
        if self.devtools {
            for l in layers.iter().rev().filter(|l| l.name != "tooltip") {
                if let Some((r, _, p)) = l.all.iter().rev().find(|(r, _, _)| contains(*r, input.mouse.0, input.mouse.1))
                {
                    let mut n = &l.roots[p[0]];
                    for &i in &p[1..] {
                        n = &n.children[i];
                    }
                    inspect = Some(InspectInfo {
                        id: n.id.as_deref().unwrap_or("").to_string(),
                        owner: n.owner.to_string(),
                        kind: format!("{:?}", n.kind).to_lowercase(),
                        rect: *r,
                        path: format!("{}:{:?}", l.name, p),
                    });
                    draw.push(Draw::Outline { rect: *r, color: [1.0, 0.85, 0.2, 0.95], width: 2.0, radius: 0.0 });
                    break;
                }
            }
            let mut tree = Vec::new();
            for (id, n) in &self.last_trees {
                let _ = id;
                flatten(n, 0, &mut tree);
            }
            self.info.tree = tree;
        } else {
            self.info.tree.clear();
        }
        self.info.inspect = inspect;
        if rebuilt {
            self.info.build_us = self.info.build_us * 0.9 + build_us * 0.1;
        }
        self.info.layout_us = self.info.layout_us * 0.9 + layout_us * 0.1;
        self.info.paint_us = self.info.paint_us * 0.9 + paint_us * 0.1;
        self.info.nodes = nodes;
        self.info.layouts = layouts;
        self.layers = layers;
        self.ids = ids;
        self.id_keys = id_keys;
        // A requested focus lands once the node exists in a layout.
        if let Some(id) = self.focus_pending.take() {
            match self.id_keys.get(&id) {
                Some(k) => {
                    self.focused = Some(*k);
                    self.edits.entry(id).or_default();
                }
                None => self.focus_pending = Some(id),
            }
        }
        self.built = built;
        self.built_wins = built_wins;
        actions.extend(self.vm.take_actions());
        out.actions = actions;
        out.draw = draw;
        out
    }

    /// Give the keyboard back to the game (Escape).
    pub fn blur(&mut self) {
        self.focused = None;
    }

    /// Devtools: toggle layout-box outlines.
    pub fn toggle_outlines(&mut self) {
        self.info.outlines = !self.info.outlines;
    }

    /// Lay out a small tree (label, tooltip, popup) at its natural size,
    /// cached by content: the same label next frame costs a hash.
    fn place_small(&mut self, n: &Node, max: (f32, f32), at: impl FnOnce((f32, f32)) -> (f32, f32)) -> Vec<Rect> {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        n.layout_hash(&mut h, &mut self.text);
        h.write_u32(max.0.to_bits());
        h.write_u32(max.1.to_bits());
        let key = h.finish();
        if !self.small.contains_key(&key) {
            if self.small.len() > 4000 {
                self.small.clear();
            }
            let size = self.lay.natural_size(n, max, &mut self.text);
            let rects = self.lay.layout(n, size, (0.0, 0.0), &mut self.text);
            self.small.insert(key, (size, rects));
        }
        let (size, rects) = &self.small[&key];
        let (x, y) = at(*size);
        rects.iter().map(|r| [r[0] + x, r[1] + y, r[2], r[3]]).collect()
    }

    fn layout_cached(
        &mut self,
        name: &str,
        root: &Node,
        avail: (f32, f32),
        origin: (f32, f32),
        count: &mut u64,
    ) -> Vec<Rect> {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        root.layout_hash(&mut h, &mut self.text);
        let hash = h.finish();
        if let Some((ch, ca, rects)) = self.cache.get(name) {
            if *ch == hash && *ca == avail {
                return rects.clone();
            }
        }
        *count += 1;
        let rects = self.lay.layout(root, avail, origin, &mut self.text);
        self.cache.insert(name.to_string(), (hash, avail, rects.clone()));
        rects
    }

    fn clamp_scroll(&mut self, layers: &[LayerOut]) {
        for l in layers {
            for h in l.hits.iter().filter(|h| h.scroll) {
                let mut n = &l.roots[h.path[0]];
                for &i in &h.path[1..] {
                    n = &n.children[i];
                }
                // One layout of the scroll area gives taffy's content size,
                // instead of measuring every child separately.
                let content = self.lay.content_height(n, (h.rect[2], h.rect[3]), &mut self.text);
                let max = (content - h.rect[3]).max(0.0);
                if let Some(v) = self.scroll.get_mut(&h.key) {
                    *v = v.min(max);
                }
            }
        }
    }

    /// The docking shell: regions at the screen edges stack their panels.
    fn shell(&self, built: &[(vm::Mount, Node)], screen: (f32, f32)) -> Node {
        let group = |region: &str, align: Option<&str>| -> Vec<Node> {
            let mut v: Vec<(&vm::Mount, &Node)> = built
                .iter()
                .filter(|(m, _)| m.layer == region && align.is_none_or(|a| m.align == a))
                .map(|(m, n)| (m, n))
                .collect();
            v.sort_by_key(|(m, _)| m.order);
            v.into_iter().map(|(_, n)| n.clone()).collect()
        };
        let gap = self.theme.space.get("s").copied().unwrap_or(4.0) * self.theme.scale;
        let side = |region: &str, key: u64| -> Node {
            let start = plain(key + 1, Style { gap, ..Default::default() }, group(region, Some("start")));
            let end = plain(
                key + 2,
                Style { gap, justify: Some(node::Align::End), ..Default::default() },
                group(region, Some("end")),
            );
            let spacer = plain(key + 3, Style { grow: 1.0, ..Default::default() }, vec![]);
            plain(
                key,
                Style { h: Len::Frac(1.0), gap, pad: [gap, gap, gap, gap], ..Default::default() },
                vec![start, spacer, end],
            )
        };
        let top = plain(10, Style { w: Len::Frac(1.0), ..Default::default() }, group("top", None));
        let bottom = plain(20, Style { w: Len::Frac(1.0), ..Default::default() }, group("bottom", None));
        let center = plain(30, Style { grow: 1.0, ..Default::default() }, vec![]);
        let middle = plain(
            40,
            Style { row: true, grow: 1.0, w: Len::Frac(1.0), align: Some(node::Align::Stretch), ..Default::default() },
            vec![side("left", 50), center, side("right", 60)],
        );
        plain(1, Style { w: Len::Px(screen.0), h: Len::Px(screen.1), ..Default::default() }, vec![top, middle, bottom])
    }

    /// Place anchored nodes at their entities or cells, nudging overlaps apart.
    fn place_anchored(
        &mut self,
        tree: &Node,
        world: &rim_sim::world::World,
        client: &ClientView,
    ) -> Vec<(Node, Vec<Rect>)> {
        let mut items: Vec<&Node> = Vec::new();
        fn gather<'a>(n: &'a Node, out: &mut Vec<&'a Node>) {
            if n.kind == Kind::Anchored {
                out.push(n);
            } else {
                for c in &n.children {
                    gather(c, out);
                }
            }
        }
        gather(tree, &mut items);
        items.sort_by_key(|n| std::cmp::Reverse(n.priority));
        let (sw, sh) = client.screen;
        let gap = 6.0 * self.theme.scale;
        let mut taken: Vec<Rect> = Vec::new();
        let mut out = Vec::new();
        // Only the top few by priority get placed: a crowd of two hundred
        // names is unreadable anyway, and placing them all was the frame.
        let mut placed = 0usize;
        for n in items {
            if placed >= ANCHOR_CAP {
                break;
            }
            let Some(anchor) = n.anchor else { continue };
            let (ax, ay) = match anchor {
                Anchor::Entity(bits) => {
                    let Some(e) = rim_sim::hecs::Entity::from_bits(bits) else { continue };
                    let Ok(p) = world.ecs.get::<&rim_sim::world::Pawn>(e) else { continue };
                    view::pawn_screen(&p, client)
                }
                Anchor::Cell(x, y) => view::cell_screen(x as f32 + 0.5, y as f32 + 0.5, client),
            };
            if ax < -200.0 || ay < -200.0 || ax > sw + 200.0 || ay > sh + 200.0 {
                continue;
            }
            placed += 1;
            let mut wrapper = n.clone();
            wrapper.kind = Kind::Box;
            let rel = self.place_small(&wrapper, (sw * 0.3, sh * 0.3), |_| (0.0, 0.0));
            let (w, h) = (rel[0][2], rel[0][3]);
            let base_y = if n.offset_y < 0.0 { ay + n.offset_y - h } else { ay + n.offset_y };
            let x = ax - w / 2.0;
            // Try the spot itself, then step away from the anchor until free.
            // If a crowd leaves no room, the lower-priority label is dropped:
            // labels never draw on top of each other.
            let dir = if n.offset_y < 0.0 { -1.0 } else { 1.0 };
            let free = (0..ANCHOR_SLOTS).map(|step| [x, base_y + dir * step as f32 * (h + gap), w, h]).find(|r| {
                !taken
                    .iter()
                    .any(|t| r[0] < t[0] + t[2] && t[0] < r[0] + r[2] && r[1] < t[1] + t[3] && t[1] < r[1] + r[3])
            });
            let Some(rect) = free else { continue };
            taken.push(rect);
            let rects = rel.iter().map(|r| [r[0] + rect[0], r[1] + rect[1], r[2], r[3]]).collect();
            out.push((wrapper, rects));
        }
        out
    }

    /// The hovered node's tooltip, once it has been hovered long enough.
    pub fn tooltip_text(&self, now: f64) -> Option<String> {
        let key = self.hovered?;
        if now - self.hovered_since < TOOLTIP_DELAY {
            return None;
        }
        self.layers.iter().find_map(|l| {
            let h = l.hits.iter().find(|h| h.key == key)?;
            let mut n = l.roots.get(h.path[0])?;
            for &i in &h.path[1..] {
                n = n.children.get(i)?;
            }
            n.tooltip.clone()
        })
    }

    fn tooltip_node(&self, tip: &str) -> Node {
        let th = &self.theme;
        let pad = th.space(&Token::Name("s".into())).unwrap_or(4.0);
        let text = Node {
            text: Some(TextStyle {
                text: tip.to_string(),
                size: th.text_size(&Token::Name("small".into())).unwrap_or(11.0),
                weight: th.weight(&Token::Name("regular".into())).unwrap_or(400),
                color: th.color("text").unwrap_or([1.0; 4]),
                wrap: true,
            }),
            kind: Kind::Text,
            ..plain(3, Style::default(), vec![])
        };
        plain(
            2,
            Style {
                pad: [pad, pad * 2.0, pad, pad * 2.0],
                bg: th.color("surface_raised").ok().or(th.color("surface").ok()),
                border: th.color("line").ok(),
                border_w: th.scale.max(1.0),
                radius: th.shape(&Token::Name("radius".into())).unwrap_or(0.0),
                max_w: Some(320.0 * th.scale),
                ..Default::default()
            },
            vec![text],
        )
    }

    /// Snapshot of every built tree, for tests.
    pub fn snapshot(&self) -> String {
        self.last_trees.iter().map(|(id, n)| format!("== {id}\n{}", n.snapshot())).collect()
    }

    pub fn mod_dirs(&self) -> &[ModDir] {
        &self.mods
    }
}

/// An engine-built container node.
fn plain(key: u64, style: Style, children: Vec<Node>) -> Node {
    Node {
        kind: Kind::Box,
        id: None,
        aka: None,
        owner: Rc::from("rim"),
        key: key.wrapping_mul(0x9E37_79B9_7F4A_7C15),
        style,
        text: None,
        hover: None,
        press: None,
        focus: None,
        disabled: false,
        on_click: None,
        on_right_click: None,
        tooltip: None,
        focusable: false,
        anchor: None,
        priority: 0,
        offset_y: 0.0,
        grid: None,
        handle: None,
        image: None,
        input: None,
        on_drag: None,
        children,
    }
}

fn collect_nodes(
    n: &Node,
    rects: &[Rect],
    i: &mut usize,
    path: &mut Vec<usize>,
    f: &mut impl FnMut(&Node, Rect, &[usize]),
) {
    let r = rects[*i];
    *i += 1;
    f(n, r, path);
    for (ci, c) in n.children.iter().enumerate() {
        path.push(ci);
        collect_nodes(c, rects, i, path, f);
        path.pop();
    }
}

fn flatten(n: &Node, depth: usize, out: &mut Vec<(usize, String, String, String)>) {
    let kind = match (n.kind, n.style.row) {
        (Kind::Box, true) => "row",
        (Kind::Box, false) => "col",
        (Kind::Text, _) => "text",
        (Kind::Spacer, _) => "spacer",
        (Kind::Scroll, _) => "scroll",
        (Kind::Anchored, _) => "anchored",
        (Kind::Grid, _) => "grid",
        (Kind::Image, _) => "image",
        (Kind::Input, _) => "input",
    };
    out.push((depth, kind.to_string(), n.id.as_deref().unwrap_or("").to_string(), n.owner.to_string()));
    for c in &n.children {
        flatten(c, depth + 1, out);
    }
}

/// The UI directories of loaded mods (for `Ui::new`).
pub fn mods_of(sim: &rim_sim::Sim) -> Vec<ModDir> {
    vm::mod_dirs(&sim.mods)
}

/// Every font a mod ships under `ui/fonts/` (TrueType, OpenType or a
/// collection), in mod load order, then by name.
pub fn mod_fonts(mods: &[(String, &std::path::Path)]) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for (_, dir) in mods {
        let Ok(rd) = std::fs::read_dir(dir.join("ui").join("fonts")) else { continue };
        let mut files: Vec<std::path::PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| ["ttf", "otf", "ttc"].iter().any(|x| e.eq_ignore_ascii_case(x)))
            })
            .collect();
        files.sort();
        out.extend(files);
    }
    out
}
