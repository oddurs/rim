//! The item token (DESIGN.md §4f, Showing contents): one component that
//! draws an item on every screen, from the thing's own world look, so a
//! modded item has a token without anyone drawing an icon.
//!
//! A look is layers of primitives in cell units (fill, outline, disc,
//! edges; `rim_sim::look`). Here they become `Shape`s once per thing and
//! material, cached by the VM (`view.look` hands out their index), and are
//! painted into whatever box a token or a grid cell has. The world atlas is
//! the client's, not the UI's, so a sprite layer draws as a fill in its
//! colour: the right place and tint, not the picture.

use crate::layout::Rect;
use crate::paint::Draw;
use crate::text::Text;
use crate::theme::Rgba;
use rim_sim::defs::{DefDb, DefId};
use rim_sim::look::{Layer, Prim};
use std::collections::HashMap;
use std::rc::Rc;

/// One primitive of a look, resolved to a colour, in cell units.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// A rectangle; a disc is one with its radius at half its side.
    Fill { rect: [f32; 4], round: bool, color: Rgba },
    /// A rectangle's outline, `width` logical points thick.
    Outline { rect: [f32; 4], width: f32, color: Rgba },
    /// A character centred on a point, `size` of the box high.
    Glyph { text: String, at: [f32; 2], size: f32, color: Rgba },
}

/// The shapes of a thing def's look, in its material's colour when it has
/// one (as the world draws it).
pub fn shapes(defs: &DefDb, def: DefId, made_of: Option<DefId>) -> Vec<Shape> {
    let td = defs.thing(def);
    let own = defs.thing(made_of.unwrap_or(def)).rgb;
    let layers: Vec<Layer> =
        if td.look_r.layers.is_empty() { rim_sim::look::plain() } else { td.look_r.layers.clone() };
    let mut out = Vec::new();
    for l in layers {
        let [r, g, b, a] = l.color.unwrap_or([own[0], own[1], own[2], 255]);
        let f = l.shade;
        let color = [
            (r as f32 / 255.0 * f).min(1.0),
            (g as f32 / 255.0 * f).min(1.0),
            (b as f32 / 255.0 * f).min(1.0),
            a as f32 / 255.0,
        ];
        out.push(match l.prim {
            Prim::Fill { rect, .. } | Prim::Sprite { rect, .. } => Shape::Fill { rect, round: false, color },
            Prim::Mass => Shape::Fill { rect: [0.0, 0.0, 1.0, 1.0], round: false, color },
            Prim::Outline { rect, width } => Shape::Outline { rect, width, color },
            Prim::Edges { width } => Shape::Outline { rect: [0.0, 0.0, 1.0, 1.0], width, color },
            Prim::Disc { at: [x, y], r, .. } => {
                Shape::Fill { rect: [x - r, y - r, 2.0 * r, 2.0 * r], round: true, color }
            }
            Prim::Glyph { at, size, id } => match defs.glyphs.get(id as usize) {
                Some(text) => Shape::Glyph { text: text.clone(), at, size, color },
                None => continue,
            },
            // A door's swing: detail a token's few pixels can't show, and
            // it has no line to draw one with.
            Prim::Arc { .. } => continue,
            // Hairlines a token's few pixels can't show.
            Prim::Pattern { .. } => continue,
        });
    }
    out
}

/// Looks by thing and material, made once and handed to scripts by index,
/// and the theme's token style. A different set of defs (another game's
/// mods) starts it afresh. Keyed by the ids scripts pass ("core:wood" and
/// its material), so a token already made costs one map lookup and no
/// allocation.
#[derive(Default)]
pub struct Looks {
    defs: usize,
    ids: HashMap<String, u32>,
    key: String,
    list: Vec<Rc<[Shape]>>,
    style: Option<TokenStyle>,
}

/// Count size and weight, and colours, from the theme.
pub type TokenStyle = (f32, u16, TokenColors);

impl Looks {
    /// The index of a thing's look, by the ids a script names: `thing`,
    /// and `made_of` or "". Unknown ids are an error naming them.
    pub fn id(&mut self, defs: &std::sync::Arc<DefDb>, thing: &str, made_of: &str) -> Result<u32, String> {
        let at = std::sync::Arc::as_ptr(defs) as usize;
        if at != self.defs {
            *self = Looks { defs: at, ..Default::default() };
        }
        self.key.clear();
        self.key.push_str(thing);
        self.key.push('|');
        self.key.push_str(made_of);
        if let Some(&id) = self.ids.get(self.key.as_str()) {
            return Ok(id);
        }
        let find = |id: &str| defs.thing_id(id).ok_or_else(|| format!("view.look: no thing '{id}'"));
        let def = find(thing)?;
        let of = if made_of.is_empty() { None } else { Some(find(made_of)?) };
        self.list.push(shapes(defs, def, of).into());
        let id = self.list.len() as u32 - 1;
        self.ids.insert(self.key.clone(), id);
        Ok(id)
    }

    pub fn get(&self, id: u32) -> Option<Rc<[Shape]>> {
        self.list.get(id as usize).cloned()
    }

    /// The theme's token style, worked out once a build (`new_build`).
    pub fn style(&mut self, theme: &crate::theme::Theme) -> Result<TokenStyle, String> {
        if let Some(s) = self.style {
            return Ok(s);
        }
        let s = (
            theme.size_named("text", "caption")?,
            theme.weight_named("strong")?,
            TokenColors {
                track: theme.color("track")?,
                text: theme.color("text")?,
                bad: theme.color("bad")?,
                accent: theme.color("accent")?,
                line: theme.color("line_strong")?,
                threat: theme.color("threat")?,
            },
        );
        self.style = Some(s);
        Ok(s)
    }

    /// A build is starting: the theme may have changed since the last.
    pub fn new_build(&mut self) {
        self.style = None;
    }
}

/// What the token says about its stack besides the look.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum State {
    #[default]
    Plain,
    /// A hauler has reserved this room: a dashed accent outline.
    Incoming,
    /// Here, but its store no longer takes it: a threat wash and outline.
    Leaving,
    /// A slot with nothing in it: a dashed hairline.
    Empty,
}

/// One item token, with its colours resolved from the theme when the node
/// was made.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub look: Option<Rc<[Shape]>>,
    /// Drawn bottom right; empty draws nothing.
    pub count: String,
    /// At its stack limit: a notch in the top right corner.
    pub full: bool,
    /// Condition, 0 to 1: a bar along the bottom when below 1.
    pub hp: Option<f32>,
    pub state: State,
    pub scale: f32,
    pub count_size: f32,
    pub count_weight: u16,
    pub colors: TokenColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TokenColors {
    pub track: Rgba,
    pub text: Rgba,
    pub bad: Rgba,
    pub accent: Rgba,
    pub line: Rgba,
    pub threat: Rgba,
}

fn fade(c: Rgba, a: f32) -> Rgba {
    [c[0], c[1], c[2], c[3] * a]
}

/// A dashed outline of short rects (the draw list has no dashed lines).
fn dashed(out: &mut Vec<Draw>, r: Rect, color: Rgba, width: f32, dash: f32) {
    let step = dash * 1.8;
    let mut run = |x0: f32, y0: f32, len: f32, across: bool| {
        let mut t = 0.0;
        while t < len {
            let d = dash.min(len - t);
            let rect = if across { [x0 + t, y0, d, width] } else { [x0, y0 + t, width, d] };
            out.push(Draw::Rect { rect, color, radius: 0.0 });
            t += step;
        }
    };
    run(r[0], r[1], r[2], true);
    run(r[0], r[1] + r[3] - width, r[2], true);
    run(r[0], r[1], r[3], false);
    run(r[0] + r[2] - width, r[1], r[3], false);
}

/// Paint a look's shapes over a box.
pub fn paint_look(shapes: &[Shape], r: Rect, alpha: f32, scale: f32, text: &mut Text, out: &mut Vec<Draw>) {
    let at = |rect: [f32; 4]| [r[0] + rect[0] * r[2], r[1] + rect[1] * r[3], rect[2] * r[2], rect[3] * r[3]];
    for s in shapes {
        match s {
            Shape::Fill { rect, round, color } => {
                let px = at(*rect);
                let radius = if *round { px[2].min(px[3]) / 2.0 } else { 0.0 };
                out.push(Draw::Rect { rect: px, color: fade(*color, alpha), radius });
            }
            Shape::Outline { rect, width, color } => {
                out.push(Draw::Outline {
                    rect: at(*rect),
                    color: fade(*color, alpha),
                    width: width * scale,
                    radius: 0.0,
                });
            }
            Shape::Glyph { text: g, at: [x, y], size, color } => {
                let size = size * r[3];
                let m = text.shape(g, size, 400, 0.0, None);
                let (gx, gy) = (r[0] + x * r[2] - m.width / 2.0, r[1] + y * r[3] - m.height / 2.0);
                let quads = text.quads(g, size, 400, 0.0, None, gx, gy);
                if !quads.is_empty() {
                    out.push(Draw::Glyphs { quads, color: fade(*color, alpha) });
                }
            }
        }
    }
}

/// Paint a token over a box: the ground, the look, then what it says.
pub fn paint(t: &Token, r: Rect, alpha: f32, text: &mut Text, out: &mut Vec<Draw>) {
    let c = &t.colors;
    let s = t.scale;
    let radius = (r[2] / 8.0).min(4.0 * s);
    let line = s.max(1.0);
    match t.state {
        State::Empty => {
            dashed(out, r, fade(c.line, alpha), line, 3.0 * s);
            return;
        }
        State::Leaving => {
            out.push(Draw::Rect { rect: r, color: fade(c.track, alpha), radius });
            out.push(Draw::Rect { rect: r, color: fade(c.threat, alpha * 0.22), radius });
        }
        State::Incoming | State::Plain => out.push(Draw::Rect { rect: r, color: fade(c.track, alpha), radius }),
    }
    if let Some(look) = &t.look {
        // Dimmer while it's only on its way, or on its way out.
        let a = if matches!(t.state, State::Incoming | State::Leaving) { 0.5 } else { 1.0 };
        paint_look(look, r, alpha * a, s, text, out);
    }
    match t.state {
        State::Incoming => dashed(out, r, fade(c.accent, alpha), line, 3.0 * s),
        State::Leaving => out.push(Draw::Outline { rect: r, color: fade(c.threat, alpha), width: line, radius }),
        _ => {}
    }
    if t.full {
        let n = (r[2] / 6.0).round().max(2.0);
        let notch = [r[0] + r[2] - n - line, r[1] + line, n, n];
        out.push(Draw::Rect { rect: notch, color: fade(c.text, alpha * 0.75), radius: 1.0 });
    }
    if let Some(hp) = t.hp.filter(|h| *h < 1.0) {
        let (h, inset) = ((2.0 * s).round().max(1.0), 3.0 * s);
        let track = [r[0] + inset, r[1] + r[3] - h, r[2] - 2.0 * inset, h];
        out.push(Draw::Rect { rect: track, color: fade([0.0, 0.0, 0.0, 0.4], alpha), radius: 0.0 });
        let fill = [track[0], track[1], track[2] * hp.clamp(0.0, 1.0), h];
        out.push(Draw::Rect { rect: fill, color: fade(c.bad, alpha), radius: 0.0 });
    }
    if !t.count.is_empty() {
        let m = text.shape(&t.count, t.count_size, t.count_weight, 0.0, None);
        let (x, y) = (r[0] + r[2] - m.width - 2.0 * s, r[1] + r[3] - m.height - s);
        let quads = text.quads(&t.count, t.count_size, t.count_weight, 0.0, None, x, y);
        if !quads.is_empty() {
            // A shadow a point down, so the count reads over any look.
            let mut shadow = quads.clone();
            shadow.iter_mut().for_each(|q| q.dst[1] += s);
            out.push(Draw::Glyphs { quads: shadow, color: fade([0.0, 0.0, 0.0, 0.85], alpha) });
            out.push(Draw::Glyphs { quads, color: fade(c.text, alpha) });
        }
    }
}
