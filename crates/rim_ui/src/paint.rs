//! Paint: laid-out nodes in, a draw list and hit boxes out.
//!
//! The draw list is all the renderer ever sees: rectangles, outlines,
//! glyph quads from the atlas, and clip regions, in physical pixels.

use crate::layout::Rect;
use crate::node::{Kind, Node, StatePatch};
use crate::text::{GlyphQuad, Text};
use crate::theme::Rgba;

#[derive(Clone, Debug, PartialEq)]
pub enum Draw {
    Rect {
        rect: Rect,
        color: Rgba,
        radius: f32,
    },
    Outline {
        rect: Rect,
        color: Rgba,
        width: f32,
        radius: f32,
    },
    Glyphs {
        quads: Vec<GlyphQuad>,
        color: Rgba,
    },
    /// Restrict drawing to a rectangle until the matching `Unclip`.
    Clip(Rect),
    Unclip,
}

/// Something the pointer can land on, in draw order (last is topmost).
#[derive(Clone, Debug)]
pub struct Hit {
    pub key: u64,
    pub rect: Rect,
    /// Where it's visible (after clipping by scroll areas).
    pub clip: Option<Rect>,
    /// Path of node indices from the layer root, to find the node again.
    pub path: Vec<usize>,
    pub interactive: bool,
    pub scroll: bool,
    pub focusable: bool,
}

/// Interaction state the painter needs to pick hover and press colours.
pub struct PaintState<'a> {
    pub hovered: Option<u64>,
    pub pressed: Option<u64>,
    pub focused: Option<u64>,
    pub scroll: &'a std::collections::HashMap<u64, f32>,
    pub disabled_alpha: f32,
}

pub fn intersect(a: Rect, b: Rect) -> Rect {
    let x0 = a[0].max(b[0]);
    let y0 = a[1].max(b[1]);
    let x1 = (a[0] + a[2]).min(b[0] + b[2]);
    let y1 = (a[1] + a[3]).min(b[1] + b[3]);
    [x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0)]
}

pub fn contains(r: Rect, x: f32, y: f32) -> bool {
    x >= r[0] && y >= r[1] && x < r[0] + r[2] && y < r[1] + r[3]
}

fn apply(base: Option<Rgba>, patch: Option<&StatePatch>, f: impl Fn(&StatePatch) -> Option<Rgba>) -> Option<Rgba> {
    patch.and_then(f).or(base)
}

fn fade(c: Rgba, a: f32) -> Rgba {
    [c[0], c[1], c[2], c[3] * a]
}

/// Paint a laid-out tree. `rects` is `layout`'s output for this tree.
#[allow(clippy::too_many_arguments)]
pub fn paint(
    root: &Node,
    rects: &[Rect],
    offset: (f32, f32),
    state: &PaintState,
    text: &mut Text,
    images: &crate::image::Images,
    draw: &mut Vec<Draw>,
    hits: &mut Vec<Hit>,
) {
    let mut p = Painter { state, text, images, draw, hits, i: 0, path: Vec::new() };
    walk(root, rects, offset, None, false, &mut p);
}

/// What every node needs while painting, and where the walk is: the next
/// rect to take and the path from the root.
struct Painter<'a> {
    state: &'a PaintState<'a>,
    text: &'a mut Text,
    images: &'a crate::image::Images,
    draw: &'a mut Vec<Draw>,
    hits: &'a mut Vec<Hit>,
    i: usize,
    path: Vec<usize>,
}

fn walk(
    n: &Node,
    rects: &[Rect],
    offset: (f32, f32),
    clip: Option<Rect>,
    // A disabled ancestor dims everything inside it.
    inherited_disabled: bool,
    p: &mut Painter,
) {
    let r0 = rects[p.i];
    p.i += 1;
    let rect = [r0[0] + offset.0, r0[1] + offset.1, r0[2], r0[3]];
    let hovered = p.state.hovered == Some(n.key) && !n.disabled;
    let pressed = p.state.pressed == Some(n.key) && hovered;
    let focused = p.state.focused == Some(n.key);
    let patch = if pressed {
        n.press.as_ref().or(n.hover.as_ref())
    } else if hovered {
        n.hover.as_ref()
    } else if focused {
        n.focus.as_ref()
    } else {
        None
    };
    let disabled = n.disabled || inherited_disabled;
    let alpha = if disabled { p.state.disabled_alpha } else { 1.0 };

    let s = &n.style;
    if let Some(bg) = apply(s.bg, patch, |p| p.bg) {
        p.draw.push(Draw::Rect { rect, color: fade(bg, alpha), radius: s.radius });
    }
    if let Some(b) = apply(s.border, patch, |p| p.border) {
        p.draw.push(Draw::Outline { rect, color: fade(b, alpha), width: s.border_w.max(1.0), radius: s.radius });
    }
    if let Some(t) = &n.text {
        let color = apply(Some(t.color), patch, |p| p.color).unwrap();
        let width = if t.wrap { Some(rect[2]) } else { None };
        let quads = p.text.quads(&t.text, t.size, t.weight, width, rect[0], rect[1]);
        if !quads.is_empty() {
            p.draw.push(Draw::Glyphs { quads, color: fade(color, alpha) });
        }
    }
    if let Some(img) = &n.image {
        let quad = p
            .images
            .pick(&img.name, 1.0)
            .filter(|d| d.factor == img.factor)
            .or_else(|| p.images.pick(&img.name, 2.0))
            .and_then(|d| p.text.image_quad(d, &img.name, rect, img.tint.is_some()));
        match quad {
            Some(q) => {
                let color = fade(img.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]), alpha);
                p.draw.push(Draw::Glyphs { quads: vec![q], color });
            }
            // Too big for the atlas: a plain box where it would be.
            None => p.draw.push(Draw::Rect { rect, color: fade([0.8, 0.2, 0.5, 0.6], alpha), radius: 0.0 }),
        }
    }
    if let Some(g) = &n.grid {
        // Cells are painted here, not laid out: one node's worth of tree
        // however many there are. Text sits inset from the cell's corner.
        let inset = (g.cell_h * 0.15).min(4.0);
        let default_color = n.style.border.unwrap_or([0.9, 0.9, 0.9, 1.0]);
        for r in 0..g.rows {
            for c in 0..g.cols {
                let cell = &g.cells[r * g.cols + c];
                let cr = g.cell_rect(rect, r, c);
                if let Some(bg) = cell.bg {
                    p.draw.push(Draw::Rect { rect: cr, color: fade(bg, alpha), radius: 0.0 });
                }
                if !cell.text.is_empty() {
                    let quads = p.text.quads(&cell.text, g.size, g.weight, None, cr[0] + inset, cr[1] + inset);
                    if !quads.is_empty() {
                        p.draw.push(Draw::Glyphs { quads, color: fade(cell.color.unwrap_or(default_color), alpha) });
                    }
                }
            }
        }
    }
    if n.is_interactive() || n.kind == Kind::Scroll {
        p.hits.push(Hit {
            key: n.key,
            rect,
            clip,
            path: p.path.clone(),
            interactive: n.is_interactive() && !n.disabled,
            scroll: n.kind == Kind::Scroll,
            focusable: (n.focusable || n.on_click.is_some()) && !n.disabled,
        });
    }

    let mut child_clip = clip;
    let mut child_offset = offset;
    if s.clip {
        let c = clip.map_or(rect, |c| intersect(c, rect));
        p.draw.push(Draw::Clip(c));
        child_clip = Some(c);
        if n.kind == Kind::Scroll {
            child_offset.1 -= p.state.scroll.get(&n.key).copied().unwrap_or(0.0);
        }
    }
    for (ci, c) in n.children.iter().enumerate() {
        p.path.push(ci);
        walk(c, rects, child_offset, child_clip, disabled, p);
        p.path.pop();
    }
    if s.clip {
        p.draw.push(Draw::Unclip);
        if let Some(outer) = clip {
            p.draw.push(Draw::Clip(outer));
        }
    }
}

/// Content height of a scroll node (for clamping its offset).
pub fn content_height(rects: &[Rect], start: usize, n: &Node) -> f32 {
    // Children follow the node in pre-order; their extent is what scrolls.
    let top = rects[start][1];
    let mut i = start + 1;
    let mut bottom = top;
    fn count(n: &Node) -> usize {
        1 + n.children.iter().map(count).sum::<usize>()
    }
    for c in &n.children {
        let r = rects[i];
        bottom = bottom.max(r[1] + r[3]);
        i += count(c);
    }
    bottom - top + n.style.pad[2]
}
