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
use std::collections::HashMap;
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
    /// Wall-clock seconds.
    pub time: f64,
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

/// One laid-out layer from the last frame, kept for input routing.
struct LayerOut {
    name: &'static str,
    roots: Vec<Node>,
    hits: Vec<Hit>,
    /// Rectangles that stop the pointer reaching anything below.
    solids: Vec<Rect>,
    /// Every node with its rect, for devtools and `find`.
    all: Vec<(Rect, usize, Vec<usize>)>,
}

/// A laid-out tree: its layout hash, the space it had, and its rects.
type CachedLayout = (u64, (f32, f32), Vec<Rect>);

/// Layers bottom to top.
const LAYERS: &[&str] = &["anchored", "docked", "windows", "cursor", "modal", "tooltip"];
const TOOLTIP_DELAY: f64 = 0.45;
/// How many slots away from its anchor a label may move to avoid overlap.
const ANCHOR_SLOTS: usize = 12;
/// Trees rebuild on any input or client change, and otherwise at this rate:
/// hover and press restyle at paint time and anchored labels are placed
/// every frame, so between rebuilds nothing visible goes stale.
const REBUILD_EVERY: f64 = 1.0 / 20.0;

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
    focused: Option<u64>,
    scroll: HashMap<u64, f32>,
    cache: HashMap<String, CachedLayout>,
    pub info: EngineInfo,
    pub devtools: bool,
    ids: HashMap<String, Rect>,
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
    built_at: f64,
    built_for: u64,
    /// Tree rebuilds so far (tests and the profiler).
    pub builds: u64,
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
        let text = Text::new(if theme.font.is_empty() { None } else { Some(theme.font.as_str()) })?;
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
            focused: None,
            scroll: HashMap::new(),
            cache: HashMap::new(),
            info,
            devtools: false,
            ids: HashMap::new(),
            reload_stamp,
            reload_checked: 0.0,
            reload_error: None,
            reloads: 0,
            last_trees: Vec::new(),
            shown: EngineInfo::default(),
            shown_at: f64::MIN,
            small: HashMap::new(),
            built: Vec::new(),
            built_at: f64::MIN,
            built_for: 0,
            builds: 0,
        })
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
        }
    }

    /// Rectangle of the node with this id in the last frame (autotests, devtools).
    pub fn find(&self, id: &str) -> Option<Rect> {
        self.ids.get(id).copied()
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
        let theme = Theme::load(&dirs, self.theme.scale);
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
        self.cache.clear();
        self.small.clear();
        self.reload_error = None;
        self.reloads += 1;
        true
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
    fn route(&mut self, input: &Input, out: &mut Output) -> Vec<(mlua::Function, bool)> {
        let (mx, my) = input.mouse;
        let mut handlers = Vec::new();
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

        if input.left_pressed && out.mouse_over_ui {
            out.captured_left = true;
            self.pressed = hovered;
            if let Some((_, h)) = &top {
                if h.focusable {
                    self.focused = Some(h.key);
                }
            }
        }
        if input.left_released {
            if let (Some(p), Some((layer, h))) = (self.pressed, &top) {
                if p == h.key {
                    let root = h.path[0];
                    if let Some(f) = self.node_at(layer, root, &h.path[1..]).and_then(|n| n.on_click.clone()) {
                        handlers.push((f, false));
                    }
                }
            }
            if self.pressed.is_some() {
                out.captured_left = true;
            }
            self.pressed = None;
        }
        if input.right_pressed && out.mouse_over_ui {
            out.captured_right = true;
            if let Some((layer, h)) = &top {
                if let Some(f) = self.node_at(layer, h.path[0], &h.path[1..]).and_then(|n| n.on_right_click.clone()) {
                    handlers.push((f, true));
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
        if input.enter {
            if let Some(f) = self.focused {
                let found = self
                    .layers
                    .iter()
                    .find_map(|l| l.hits.iter().find(|h| h.key == f).map(|h| (l.name, h.path.clone())));
                if let Some((layer, path)) = found {
                    if let Some(func) = self.node_at(layer, path[0], &path[1..]).and_then(|n| n.on_click.clone()) {
                        handlers.push((func, false));
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
        let handlers = self.route(input, &mut out);
        let handled = !handlers.is_empty();
        for (f, _) in handlers {
            self.vm.call_handler(&f, world, client, &self.shown);
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
            || input.tab
            || input.enter;
        let stale = input.time - self.built_at >= REBUILD_EVERY || input.time < self.built_at;
        let t0 = Instant::now();
        let rebuilt =
            handled || input_happened || stale || ch != self.built_for || self.built.is_empty() || self.devtools;
        if rebuilt {
            self.built = self.vm.build(world, client, &self.shown, &self.theme);
            self.built_at = input.time;
            self.built_for = ch;
            self.builds += 1;
            self.last_trees = self.built.iter().map(|(m, n)| (m.id.clone(), n.clone())).collect();
        }
        let build_us = t0.elapsed().as_secs_f64() * 1e6;
        let built = std::mem::take(&mut self.built);

        let (sw, sh) = client.screen;
        let mut layout_us = 0.0;
        let mut paint_us = 0.0;
        let mut layers: Vec<LayerOut> = Vec::new();
        let mut draw = Vec::new();
        let mut ids = HashMap::new();
        let mut nodes = 0;
        let mut layouts = self.info.layouts;
        let state_scroll = std::mem::take(&mut self.scroll);
        let state = PaintState {
            hovered: self.hovered,
            pressed: self.pressed,
            focused: self.focused,
            scroll: &state_scroll,
            disabled_alpha: 0.45,
        };

        for &layer in LAYERS {
            let mut lo =
                LayerOut { name: layer, roots: Vec::new(), hits: Vec::new(), solids: Vec::new(), all: Vec::new() };
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
                        let (w, h) = layout::natural_size(&n, (sw * 0.8, sh * 0.5), &mut self.text);
                        let rects =
                            layout::layout(&n, (w, h), ((sw - w) / 2.0, 40.0 * self.theme.scale), &mut self.text);
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
                paint::paint(&root, &rects, (0.0, 0.0), &state, &mut self.text, &mut draw, &mut hits);
                for mut h in hits {
                    h.path.insert(0, ri);
                    lo.hits.push(h);
                }
                let solid_layer = matches!(layer, "docked" | "windows" | "modal");
                let mut i = 0;
                let mut path = vec![ri];
                collect_nodes(&root, &rects, &mut i, &mut path, &mut |n, r, p| {
                    nodes += 1;
                    if solid_layer && (n.style.bg.is_some() || n.is_interactive()) {
                        lo.solids.push(r);
                    }
                    if let Some(id) = &n.id {
                        ids.insert(id.to_string(), r);
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
        self.built = built;
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
        n.layout_hash(&mut h);
        h.write_u32(max.0.to_bits());
        h.write_u32(max.1.to_bits());
        let key = h.finish();
        if !self.small.contains_key(&key) {
            if self.small.len() > 4000 {
                self.small.clear();
            }
            let size = layout::natural_size(n, max, &mut self.text);
            let rects = layout::layout(n, size, (0.0, 0.0), &mut self.text);
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
        root.layout_hash(&mut h);
        let hash = h.finish();
        if let Some((ch, ca, rects)) = self.cache.get(name) {
            if *ch == hash && *ca == avail {
                return rects.clone();
            }
        }
        *count += 1;
        let rects = layout::layout(root, avail, origin, &mut self.text);
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
                let content: f32 = {
                    let mut total = 0.0;
                    for c in &n.children {
                        total += layout::natural_size(c, (h.rect[2], f32::MAX), &mut self.text).1;
                    }
                    total + n.style.gap * n.children.len().saturating_sub(1) as f32 + n.style.pad[0] + n.style.pad[2]
                };
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
        let gap = 2.0 * self.theme.scale;
        let mut taken: Vec<Rect> = Vec::new();
        let mut out = Vec::new();
        for n in items {
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

    fn tooltip_text(&self, now: f64) -> Option<String> {
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
