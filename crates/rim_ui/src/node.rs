//! The UI tree: plain nodes with styles resolved from theme tokens.
//!
//! Components build Luau tables like `{ kind = "row", gap = "s", bg =
//! "surface", child1, child2 }`; `from_lua` turns them into `Node`s. Every
//! style value goes through the theme, so a typo in a token name is an error
//! that names the component rather than a silent default.

use crate::theme::{Rgba, Theme, Token};
use mlua::{Function, Table, Value};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// A part of a window's chrome the engine routes itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Handle {
    Move,
    Resize,
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Box,
    Text,
    Spacer,
    Scroll,
    /// A child attached to an entity or cell in the world (anchored layer).
    Anchored,
    /// Rows by columns of cells the engine positions and paints: one node,
    /// however many cells. See `Grid`.
    Grid,
    /// A picture from a mod's `ui/img`, drawn from the atlas. See `Image`.
    Image,
    /// A line of text the player edits; the buffer is the engine's, keyed
    /// by the node's id. See `InputData`.
    Input,
}

/// What an image node shows: the picture by name and which variant, and
/// the colour to draw it in when it is a tinted (monochrome) icon.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub name: String,
    pub factor: u32,
    pub tint: Option<Rgba>,
}

/// What a text input node carries besides the text it shows.
#[derive(Clone)]
pub struct InputData {
    /// What the input holds until the player edits it.
    pub value: String,
    /// The text shown is the placeholder (nothing typed yet).
    pub placeholder: bool,
    pub on_change: Option<Function>,
    pub on_submit: Option<Function>,
}

/// One cell of a grid, as the component's `cell(r, c)` described it.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GridCell {
    pub text: String,
    pub bg: Option<Rgba>,
    pub color: Option<Rgba>,
}

/// A grid's shape and its cells. The node is laid out as one leaf of
/// `cols * cell_w` by `rows * cell_h`; cells are painted, not laid out, so
/// a 30 by 12 board costs the tree one node.
#[derive(Clone)]
pub struct Grid {
    pub rows: usize,
    pub cols: usize,
    pub cell_w: f32,
    pub cell_h: f32,
    pub gap: f32,
    pub size: f32,
    pub weight: u16,
    pub cells: Vec<GridCell>,
    /// Called once when a drag starts, with the cell: returns the value the
    /// whole drag paints.
    pub on_press: Option<Function>,
    /// Called for the pressed cell and each newly entered one: (r, c, value).
    pub on_paint: Option<Function>,
}

impl Grid {
    /// The cell under a point inside the grid's rect, if any.
    pub fn cell_at(&self, rect: [f32; 4], x: f32, y: f32) -> Option<(usize, usize)> {
        let (dx, dy) = (x - rect[0], y - rect[1]);
        if dx < 0.0 || dy < 0.0 {
            return None;
        }
        let (c, r) = ((dx / (self.cell_w + self.gap)) as usize, (dy / (self.cell_h + self.gap)) as usize);
        (r < self.rows && c < self.cols).then_some((r, c))
    }
    pub fn cell_rect(&self, rect: [f32; 4], r: usize, c: usize) -> [f32; 4] {
        [
            rect[0] + c as f32 * (self.cell_w + self.gap),
            rect[1] + r as f32 * (self.cell_h + self.gap),
            self.cell_w,
            self.cell_h,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Len {
    #[default]
    Auto,
    Px(f32),
    /// A fraction of the parent (0..1).
    Frac(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
    Between,
}

/// Where an anchored node attaches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Anchor {
    Entity(u64),
    Cell(i32, i32),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub row: bool,
    pub gap: f32,
    /// top, right, bottom, left
    pub pad: [f32; 4],
    pub w: Len,
    pub h: Len,
    pub min_w: Option<f32>,
    pub max_w: Option<f32>,
    pub min_h: Option<f32>,
    pub max_h: Option<f32>,
    pub grow: f32,
    pub align: Option<Align>,
    pub justify: Option<Align>,
    pub bg: Option<Rgba>,
    pub border: Option<Rgba>,
    pub border_w: f32,
    pub radius: f32,
    pub clip: bool,
}

/// Style changes for hover, press and focus. Only colours: a state never
/// changes layout, so hovering can't make a panel jump.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatePatch {
    pub bg: Option<Rgba>,
    pub border: Option<Rgba>,
    pub color: Option<Rgba>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub text: String,
    pub size: f32,
    pub weight: u16,
    pub color: Rgba,
    pub wrap: bool,
}

#[derive(Clone)]
pub struct Node {
    pub kind: Kind,
    /// Namespaced id (`core:clock`), if the component gave one.
    pub id: Option<Rc<str>>,
    /// The component id, when a component's root carries its own id: the
    /// node answers to both.
    pub aka: Option<Rc<str>>,
    /// The mod whose code built this node.
    pub owner: Rc<str>,
    /// Stable key for hover, press, focus and scroll state.
    pub key: u64,
    pub style: Style,
    pub text: Option<TextStyle>,
    pub hover: Option<StatePatch>,
    pub press: Option<StatePatch>,
    pub focus: Option<StatePatch>,
    pub disabled: bool,
    pub on_click: Option<Function>,
    pub on_right_click: Option<Function>,
    pub tooltip: Option<String>,
    pub focusable: bool,
    pub anchor: Option<Anchor>,
    /// Anchored nodes: higher wins placement when labels collide.
    pub priority: i32,
    /// Anchored nodes: pixels above (negative) or below the anchor.
    pub offset_y: f32,
    /// Grid nodes: the cells.
    pub grid: Option<Rc<Grid>>,
    /// Window chrome: dragging this moves or resizes the window, clicking
    /// it closes.
    pub handle: Option<Handle>,
    /// Image nodes: the picture.
    pub image: Option<Image>,
    /// Input nodes: the buffer's seed and handlers.
    pub input: Option<InputData>,
    /// Called while the pointer is held on this node: (fx, fy) across it.
    pub on_drag: Option<Function>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn is_interactive(&self) -> bool {
        self.on_click.is_some()
            || self.on_right_click.is_some()
            || self.tooltip.is_some()
            || self.focusable
            || self.grid.as_ref().is_some_and(|g| g.on_press.is_some() || g.on_paint.is_some())
            || self.handle.is_some()
            || self.on_drag.is_some()
            || self.input.is_some()
    }

    /// Hash of everything that affects layout, for the layout cache.
    pub fn layout_hash<H: Hasher>(&self, h: &mut H) {
        (self.kind as u8).hash(h);
        let s = &self.style;
        s.row.hash(h);
        for v in [s.gap, s.pad[0], s.pad[1], s.pad[2], s.pad[3], s.grow] {
            v.to_bits().hash(h);
        }
        for l in [s.w, s.h] {
            match l {
                Len::Auto => 0u8.hash(h),
                Len::Px(v) => (1u8, v.to_bits()).hash(h),
                Len::Frac(v) => (2u8, v.to_bits()).hash(h),
            }
        }
        for v in [s.min_w, s.max_w, s.min_h, s.max_h] {
            v.map(f32::to_bits).hash(h);
        }
        s.align.hash(h);
        s.justify.hash(h);
        if let Some(t) = &self.text {
            (&t.text, t.size.to_bits(), t.weight, t.wrap).hash(h);
        }
        self.children.len().hash(h);
        for c in &self.children {
            c.layout_hash(h);
        }
    }

    /// Indented text form, for snapshot tests and devtools.
    pub fn snapshot(&self) -> String {
        let mut out = String::new();
        self.snapshot_into(&mut out, 0);
        out
    }

    fn snapshot_into(&self, out: &mut String, depth: usize) {
        out.push_str(&"  ".repeat(depth));
        let kind = match (self.kind, self.style.row) {
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
        out.push_str(kind);
        if let Some(i) = &self.image {
            out.push_str(&format!(" {}{}", i.name, if i.tint.is_some() { " tinted" } else { "" }));
        }
        if let Some(g) = &self.grid {
            out.push_str(&format!(" {}x{}", g.rows, g.cols));
        }
        if let Some(id) = &self.id {
            out.push_str(&format!(" #{id}"));
        }
        if let Some(t) = &self.text {
            out.push_str(&format!(" {:?}", t.text));
        }
        if self.on_click.is_some() {
            out.push_str(" [click]");
        }
        if self.disabled {
            out.push_str(" [disabled]");
        }
        out.push('\n');
        for c in &self.children {
            c.snapshot_into(out, depth + 1);
        }
    }
}

/// Everything conversion needs besides the table itself.
pub struct Ctx<'a> {
    pub theme: &'a Theme,
    pub owner: Rc<str>,
    pub images: &'a crate::image::Images,
    /// Text inputs' buffers, so an input shows what the player typed.
    pub edits: &'a std::collections::HashMap<String, crate::edit::EditState>,
}

/// A node with nothing in it, for engine-built containers.
pub fn blank(key: u64, owner: Rc<str>) -> Node {
    Node {
        kind: Kind::Box,
        id: None,
        aka: None,
        owner,
        key,
        style: Style::default(),
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
        children: Vec::new(),
    }
}

/// `size`, for callers outside this module.
pub fn size_of(theme: &Theme, section: &str, k: &str, v: &Value) -> Result<f32, String> {
    size(theme, section, k, v)
}

/// A size from a token name or a number, in physical pixels. Reads the
/// name straight out of Luau's string: no allocation on the common path.
fn size(theme: &Theme, section: &str, k: &str, v: &Value) -> Result<f32, String> {
    match v {
        Value::Integer(i) => Ok(*i as f32 * theme.scale),
        Value::Number(n) => Ok(*n as f32 * theme.scale),
        Value::String(s) => theme.size_named(section, &s.to_str().map_err(|e| e.to_string())?),
        _ => Err(format!("'{k}' must be a token name or a number")),
    }
}

fn weight(theme: &Theme, v: &Value) -> Result<u16, String> {
    match v {
        Value::Integer(i) => Ok(*i as u16),
        Value::Number(n) => Ok(*n as u16),
        Value::String(s) => theme.weight_named(&s.to_str().map_err(|e| e.to_string())?),
        _ => Err("'weight' must be a token name or a number".into()),
    }
}

fn string(k: &str, v: &Value) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.to_str().map_err(|e| e.to_string())?.to_string()),
        Value::Integer(i) => Ok(i.to_string()),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err(format!("'{k}' must be a string")),
    }
}

fn num(k: &str, v: &Value) -> Result<f32, String> {
    match v {
        Value::Integer(i) => Ok(*i as f32),
        Value::Number(n) => Ok(*n as f32),
        _ => Err(format!("'{k}' must be a number")),
    }
}

fn len(theme: &Theme, k: &str, v: &Value) -> Result<Len, String> {
    if let Value::String(s) = v {
        let s = s.to_str().map_err(|e| e.to_string())?;
        if &*s == "fill" {
            return Ok(Len::Frac(1.0));
        }
        if let Some(p) = s.strip_suffix('%') {
            let pct: f32 = p.parse().map_err(|_| format!("bad percentage '{}'", &*s))?;
            return Ok(Len::Frac(pct / 100.0));
        }
    }
    Ok(Len::Px(size(theme, "space", k, v)?))
}

fn align(s: &str) -> Result<Align, String> {
    Ok(match s {
        "start" => Align::Start,
        "center" => Align::Center,
        "end" => Align::End,
        "stretch" => Align::Stretch,
        "between" => Align::Between,
        _ => return Err(format!("unknown alignment '{s}'")),
    })
}

fn color(theme: &Theme, k: &str, v: &Value) -> Result<Rgba, String> {
    match v {
        Value::String(s) => theme.color(&s.to_str().map_err(|e| e.to_string())?),
        _ => Err(format!("'{k}' must be a colour token or #rrggbb")),
    }
}

fn patch(theme: &Theme, v: &Value) -> Result<StatePatch, String> {
    let Value::Table(p) = v else { return Err("state styles must be tables".into()) };
    let mut out = StatePatch::default();
    for pair in p.pairs::<String, Value>() {
        let (k, v) = pair.map_err(|e| e.to_string())?;
        match k.as_str() {
            "bg" => out.bg = Some(color(theme, "bg", &v)?),
            "border" => out.border = Some(color(theme, "border", &v)?),
            "color" => out.color = Some(color(theme, "color", &v)?),
            _ => return Err(format!("state styles take bg, border and color, not '{k}'")),
        }
    }
    Ok(out)
}

fn mix(key: u64, salt: u64) -> u64 {
    let mut z = key ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Stable key for a node: its id if it has one, else its parent's key and
/// position. Ids make state survive reordering.
pub fn key_for(parent: u64, index: usize, id: Option<&str>) -> u64 {
    match id {
        Some(id) => {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            id.hash(&mut h);
            mix(h.finish(), 1)
        }
        None => mix(parent, index as u64 + 2),
    }
}

/// Convert one node table (not its children: the caller walks those so it
/// can apply mod operations to each). One pass over the table's fields.
pub fn node_from_table(ctx: &Ctx, t: &Table, key: u64) -> Result<Node, String> {
    let theme = ctx.theme;
    let mut kind = Kind::Box;
    // Field order in a table is not fixed, and `cell` means a position on an
    // anchored node but the cell function on a grid: read the kind first.
    let is_grid = matches!(t.raw_get::<Value>("kind"), Ok(Value::String(s)) if s.as_bytes().as_ref() == b"grid");
    let mut style = Style::default();
    let mut text: Option<String> = None;
    let mut text_size = None;
    let mut text_weight = None;
    let mut text_color = None;
    let mut wrap = false;
    let mut has_radius = false;
    let mut entity: Option<u64> = None;
    let mut cell: Option<(i32, i32)> = None;
    let (mut rows, mut cols) = (0usize, 0usize);
    let (mut cell_w, mut cell_h) = (None, None);
    let mut cell_fn: Option<Function> = None;
    let (mut on_press, mut on_paint) = (None, None);
    let mut src: Option<String> = None;
    let mut tint = false;
    let mut value = String::new();
    let mut placeholder = String::new();
    let (mut on_change, mut on_submit) = (None, None);
    let mut n = Node {
        kind: Kind::Box,
        id: None,
        aka: None,
        owner: ctx.owner.clone(),
        key,
        style: Style::default(),
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
        children: Vec::new(),
    };
    for pair in t.pairs::<Value, Value>() {
        let (k, v) = pair.map_err(|e| e.to_string())?;
        let k = match k {
            Value::String(s) => s,
            Value::Integer(1) => {
                // A text node's content is its first positional value. A
                // `false` or nil there is a child left out (`cond and {...}`).
                if !matches!(v, Value::Table(_) | Value::Boolean(false) | Value::Nil) {
                    text = Some(string("text", &v)?);
                }
                continue;
            }
            _ => continue,
        };
        let k = k.to_str().map_err(|e| e.to_string())?;
        match &*k {
            "kind" => {
                (kind, style.row) = match string("kind", &v)?.as_str() {
                    "row" => (Kind::Box, true),
                    "col" | "box" => (Kind::Box, false),
                    "text" => (Kind::Text, false),
                    "spacer" => (Kind::Spacer, false),
                    "scroll" => (Kind::Scroll, false),
                    "anchored" => (Kind::Anchored, false),
                    "grid" => (Kind::Grid, false),
                    "image" => (Kind::Image, false),
                    "input" => (Kind::Input, false),
                    other => return Err(format!("unknown node kind '{other}'")),
                }
            }
            "dir" => style.row = string("dir", &v)? == "row",
            "gap" => style.gap = size(theme, "space", "gap", &v)?,
            "pad" => style.pad = [size(theme, "space", "pad", &v)?; 4],
            "padx" => {
                let p = size(theme, "space", "padx", &v)?;
                style.pad[1] = p;
                style.pad[3] = p;
            }
            "pady" => {
                let p = size(theme, "space", "pady", &v)?;
                style.pad[0] = p;
                style.pad[2] = p;
            }
            "w" => style.w = len(theme, "w", &v)?,
            "h" => style.h = len(theme, "h", &v)?,
            "minw" => style.min_w = Some(size(theme, "space", "minw", &v)?),
            "maxw" => style.max_w = Some(size(theme, "space", "maxw", &v)?),
            "minh" => style.min_h = Some(size(theme, "space", "minh", &v)?),
            "maxh" => style.max_h = Some(size(theme, "space", "maxh", &v)?),
            "grow" => style.grow = num("grow", &v)?,
            "align" => style.align = Some(align(&string("align", &v)?)?),
            "justify" => style.justify = Some(align(&string("justify", &v)?)?),
            "bg" => style.bg = Some(color(theme, "bg", &v)?),
            "border" => style.border = Some(color(theme, "border", &v)?),
            "radius" => {
                style.radius = size(theme, "shape", "radius", &v)?;
                has_radius = true;
            }
            "clip" => style.clip = matches!(v, Value::Boolean(true)),
            "text" => text = Some(string("text", &v)?),
            "size" => text_size = Some(size(theme, "text", "size", &v)?),
            "weight" => text_weight = Some(weight(theme, &v)?),
            "color" => text_color = Some(color(theme, "color", &v)?),
            "wrap" => wrap = matches!(v, Value::Boolean(true)),
            // Entity ids are 64-bit: never round-trip them through f32.
            "entity" => {
                entity = Some(match v {
                    Value::Integer(i) => i as u64,
                    Value::Number(n) => n as u64,
                    _ => return Err("'entity' must be an entity id".into()),
                })
            }
            // `cell` is a position on an anchored node and the cell function
            // on a grid.
            "cell" if is_grid => cell_fn = Some(function("cell", v)?),
            "cell" => {
                let Value::Table(c) = v else { return Err("cell must be {x, y}".into()) };
                cell = Some((c.get(1).map_err(|e| e.to_string())?, c.get(2).map_err(|e| e.to_string())?));
            }
            "id" => n.id = Some(Rc::from(string("id", &v)?)),
            "hover" => n.hover = Some(patch(theme, &v)?),
            "press" => n.press = Some(patch(theme, &v)?),
            "focus" => n.focus = Some(patch(theme, &v)?),
            "disabled" => n.disabled = matches!(v, Value::Boolean(true)),
            "on_click" => n.on_click = Some(function("on_click", v)?),
            "on_right_click" => n.on_right_click = Some(function("on_right_click", v)?),
            "tooltip" => n.tooltip = Some(string("tooltip", &v)?),
            "focusable" => n.focusable = matches!(v, Value::Boolean(true)),
            "priority" => n.priority = num("priority", &v)? as i32,
            "offset" => n.offset_y = num("offset", &v)? * theme.scale,
            "rows" => rows = num("rows", &v)?.max(0.0) as usize,
            "cols" => cols = num("cols", &v)?.max(0.0) as usize,
            "cell_w" => cell_w = Some(size(theme, "space", "cell_w", &v)?),
            "cell_h" => cell_h = Some(size(theme, "space", "cell_h", &v)?),
            "on_press" => on_press = Some(function("on_press", v)?),
            "on_paint" => on_paint = Some(function("on_paint", v)?),
            "value" => value = string("value", &v)?,
            "placeholder" => placeholder = string("placeholder", &v)?,
            "on_change" => on_change = Some(function("on_change", v)?),
            "on_submit" => on_submit = Some(function("on_submit", v)?),
            "on_drag" => n.on_drag = Some(function("on_drag", v)?),
            "handle" => {
                n.handle = Some(match string("handle", &v)?.as_str() {
                    "move" => Handle::Move,
                    "resize" => Handle::Resize,
                    "close" => Handle::Close,
                    other => return Err(format!("unknown handle '{other}' (move, resize or close)")),
                })
            }
            "src" => src = Some(string("src", &v)?),
            "tint" => tint = matches!(v, Value::Boolean(true)),
            other => return Err(format!("unknown property '{other}'")),
        }
    }
    if kind == Kind::Spacer && style.grow == 0.0 {
        style.grow = 1.0;
    }
    if style.border.is_some() {
        style.border_w = theme.size_named("shape", "border").unwrap_or(theme.scale);
    }
    if !has_radius && (style.bg.is_some() || style.border.is_some()) {
        style.radius = theme.size_named("shape", "radius").unwrap_or(0.0);
    }
    if kind == Kind::Scroll {
        style.clip = true;
    }
    if kind == Kind::Input {
        let Some(id) = n.id.as_deref() else { return Err("an input needs an id (its text is kept by it)".into()) };
        let shown = ctx.edits.get(id).map(|e| e.text.clone()).unwrap_or_else(|| value.clone());
        let empty = shown.is_empty();
        // An empty box still needs a line's height: measure a space.
        let text = if !empty {
            shown
        } else if placeholder.is_empty() {
            " ".to_string()
        } else {
            placeholder.clone()
        };
        n.text = Some(TextStyle {
            text,
            size: match text_size {
                Some(s) => s,
                None => theme.size_named("text", "body")?,
            },
            weight: match text_weight {
                Some(w) => w,
                None => theme.weight_named("regular")?,
            },
            color: match (empty, text_color) {
                (true, _) => theme.color("muted")?,
                (false, Some(c)) => c,
                (false, None) => theme.color("text")?,
            },
            wrap: false,
        });
        n.input = Some(InputData { value, placeholder: empty, on_change, on_submit });
        n.focusable = true;
    }
    if kind == Kind::Text {
        n.text = Some(TextStyle {
            text: text.unwrap_or_default(),
            size: match text_size {
                Some(s) => s,
                None => theme.size_named("text", "body")?,
            },
            weight: match text_weight {
                Some(w) => w,
                None => theme.weight_named("regular")?,
            },
            color: match text_color {
                Some(c) => c,
                None => theme.color("text")?,
            },
            wrap,
        });
    }
    if kind == Kind::Grid {
        let (Some(cw), Some(ch)) = (cell_w, cell_h) else { return Err("a grid needs cell_w and cell_h".into()) };
        let Some(cell_fn) = cell_fn else { return Err("a grid needs cell(r, c)".into()) };
        let tsize = match text_size {
            Some(s) => s,
            None => theme.size_named("text", "small")?,
        };
        let tweight = match text_weight {
            Some(w) => w,
            None => theme.weight_named("regular")?,
        };
        let mut cells = Vec::with_capacity(rows * cols);
        for r in 0..rows {
            for c in 0..cols {
                let v: Value = cell_fn.call((r as i64 + 1, c as i64 + 1)).map_err(|e| e.to_string())?;
                cells.push(match v {
                    Value::Nil => GridCell::default(),
                    Value::String(s) => {
                        GridCell { text: s.to_str().map_err(|e| e.to_string())?.to_string(), ..Default::default() }
                    }
                    Value::Integer(i) => GridCell { text: i.to_string(), ..Default::default() },
                    Value::Number(x) => GridCell { text: x.to_string(), ..Default::default() },
                    Value::Table(ct) => {
                        let text = match ct.get::<Value>("text").map_err(|e| e.to_string())? {
                            Value::Nil => match ct.get::<Value>(1).map_err(|e| e.to_string())? {
                                Value::Nil => String::new(),
                                v => string("text", &v)?,
                            },
                            v => string("text", &v)?,
                        };
                        let bg = match ct.get::<Value>("bg").map_err(|e| e.to_string())? {
                            Value::Nil => None,
                            v => Some(color(theme, "bg", &v)?),
                        };
                        let col = match ct.get::<Value>("color").map_err(|e| e.to_string())? {
                            Value::Nil => None,
                            v => Some(color(theme, "color", &v)?),
                        };
                        GridCell { text, bg, color: col }
                    }
                    _ => return Err(format!("cell({}, {}) must return text, a table or nil", r + 1, c + 1)),
                });
            }
        }
        let gap = style.gap;
        style.w = Len::Px((cols as f32 * (cw + gap) - gap).max(0.0));
        style.h = Len::Px((rows as f32 * (ch + gap) - gap).max(0.0));
        n.grid = Some(Rc::new(Grid {
            rows,
            cols,
            cell_w: cw,
            cell_h: ch,
            gap,
            size: tsize,
            weight: tweight,
            cells,
            on_press,
            on_paint,
        }));
        n.text = None;
    }
    if kind == Kind::Image {
        let Some(name) = src else { return Err("an image needs src = \"mod:name\"".into()) };
        let Some(img) = ctx.images.pick(&name, theme.scale) else {
            let mut known = ctx.images.names().join(", ");
            if known.is_empty() {
                known = "no mod ships one".into();
            }
            return Err(format!(
                "image '{name}' not found: ui/img/{}.png in a mod ({known})",
                name.rsplit(':').next().unwrap_or(&name)
            ));
        };
        // Its own logical size unless the node says otherwise.
        let f = img.factor as f32;
        if style.w == Len::Auto {
            style.w = Len::Px(img.w as f32 / f * theme.scale);
        }
        if style.h == Len::Auto {
            style.h = Len::Px(img.h as f32 / f * theme.scale);
        }
        let tint_color = if tint {
            Some(match text_color {
                Some(c) => c,
                None => theme.color("text")?,
            })
        } else {
            None
        };
        n.image = Some(Image { name, factor: img.factor, tint: tint_color });
        n.text = None;
    }
    if kind == Kind::Anchored {
        n.anchor = Some(match (entity, cell) {
            (Some(e), _) => Anchor::Entity(e),
            (None, Some((x, y))) => Anchor::Cell(x, y),
            _ => return Err("anchored node needs an entity or a cell".into()),
        });
    }
    n.kind = kind;
    n.style = style;
    Ok(n)
}

fn function(k: &str, v: Value) -> Result<Function, String> {
    match v {
        Value::Function(f) => Ok(f),
        _ => Err(format!("'{k}' must be a function")),
    }
}

/// A node that stands in for a component that failed: red, with the error.
pub fn error_node(theme: &Theme, owner: Rc<str>, key: u64, what: &str, err: &str) -> Node {
    let bg = theme.color("threat").unwrap_or([0.8, 0.2, 0.2, 1.0]);
    let size = theme.text_size(&Token::Name("small".into())).unwrap_or(11.0 * theme.scale);
    let text = Node {
        kind: Kind::Text,
        id: None,
        aka: None,
        owner: owner.clone(),
        key: mix(key, 99),
        style: Style::default(),
        text: Some(TextStyle {
            text: format!("{what}: {err}"),
            size,
            weight: 400,
            color: [1.0, 1.0, 1.0, 1.0],
            wrap: true,
        }),
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
        children: vec![],
    };
    let pad = 4.0 * theme.scale;
    Node {
        kind: Kind::Box,
        id: None,
        aka: None,
        owner,
        key,
        style: Style {
            bg: Some([bg[0], bg[1], bg[2], 0.9]),
            pad: [pad; 4],
            max_w: Some(420.0 * theme.scale),
            ..Default::default()
        },
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
        children: vec![text],
    }
}
