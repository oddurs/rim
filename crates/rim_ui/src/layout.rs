//! Layout: a node tree in, one rectangle per node (pre-order) out.
//!
//! Flexbox via taffy; text leaves are measured through the shaping cache.
//! The caller caches results by the tree's layout hash and the available
//! size, so an unchanged frame skips layout entirely.

use crate::node::{Align, Kind, Len, Node};
use crate::text::Text;
use taffy::prelude::*;
use taffy::style::ExpandedDimension;
use taffy::{Overflow, Point};

/// x, y, w, h in physical pixels.
pub type Rect = [f32; 4];

fn dim(l: Len) -> Dimension {
    match l {
        Len::Auto => Dimension::auto(),
        Len::Px(v) => Dimension::length(v),
        Len::Frac(f) => Dimension::percent(f),
    }
}

fn lpa(v: Option<f32>) -> LengthPercentageAuto {
    v.map_or(LengthPercentageAuto::auto(), LengthPercentageAuto::length)
}

fn align_items(a: Align) -> AlignItems {
    match a {
        Align::Start => AlignItems::FLEX_START,
        Align::Center => AlignItems::CENTER,
        Align::End => AlignItems::FLEX_END,
        Align::Stretch | Align::Between => AlignItems::STRETCH,
    }
}

fn justify(a: Align) -> JustifyContent {
    match a {
        Align::Start => JustifyContent::FLEX_START,
        Align::Center => JustifyContent::CENTER,
        Align::End => JustifyContent::FLEX_END,
        Align::Stretch | Align::Between => JustifyContent::SPACE_BETWEEN,
    }
}

/// What a text leaf needs to be measured.
struct Measure {
    text: String,
    size: f32,
    weight: u16,
    wrap: bool,
}

fn build(taffy: &mut TaffyTree<Option<Measure>>, n: &Node) -> NodeId {
    let s = &n.style;
    let style = Style {
        flex_direction: if s.row { FlexDirection::Row } else { FlexDirection::Column },
        gap: Size { width: LengthPercentage::length(s.gap), height: LengthPercentage::length(s.gap) },
        padding: taffy::Rect {
            top: LengthPercentage::length(s.pad[0]),
            right: LengthPercentage::length(s.pad[1]),
            bottom: LengthPercentage::length(s.pad[2]),
            left: LengthPercentage::length(s.pad[3]),
        },
        size: Size { width: dim(s.w), height: dim(s.h) },
        min_size: Size { width: lpa(s.min_w), height: lpa(s.min_h) },
        max_size: Size { width: lpa(s.max_w), height: lpa(s.max_h) },
        flex_grow: s.grow,
        // Text and grids never shrink below their content; boxes may.
        flex_shrink: if matches!(n.kind, Kind::Text | Kind::Grid | Kind::Image) { 0.0 } else { 1.0 },
        align_items: s.align.map(align_items),
        justify_content: s.justify.map(justify),
        overflow: if n.kind == Kind::Scroll {
            Point { x: Overflow::Visible, y: Overflow::Scroll }
        } else {
            Point { x: Overflow::Visible, y: Overflow::Visible }
        },
        scrollbar_width: 0.0,
        ..Default::default()
    };
    if let Some(t) = &n.text {
        let m = Measure { text: t.text.clone(), size: t.size, weight: t.weight, wrap: t.wrap };
        return taffy.new_leaf_with_context(style, Some(m)).unwrap();
    }
    let kids: Vec<NodeId> = n.children.iter().map(|c| build(taffy, c)).collect();
    taffy.new_with_children(style, &kids).unwrap()
}

/// The layout engine: one taffy tree, cleared and refilled for each call so
/// its node storage is reused instead of reallocated every frame.
pub struct Engine {
    taffy: TaffyTree<Option<Measure>>,
}

impl Default for Engine {
    fn default() -> Self {
        Engine { taffy: TaffyTree::new() }
    }
}

/// Measure a text leaf (or size a childless box) for taffy. `wrap_at` is the
/// width wrapped text gets when taffy offers no definite width.
fn measure(
    text: &mut Text,
    input: taffy::LayoutInput,
    ctx: Option<&mut Option<Measure>>,
    style: &Style,
    wrap_at: Option<f32>,
    fit: bool,
) -> taffy::LayoutOutput {
    let known = input.known_dimensions;
    // Childless boxes (spacers) have no content of their own, but once flex
    // has sized them the answer must be that size.
    // A childless root (a bare grid on the windows layer) reaches here with
    // nothing known, so its own fixed size is the answer then.
    let Some(Some(m)) = ctx else {
        let fixed = |d: Dimension| match d.expand() {
            ExpandedDimension::Length(v) => v,
            _ => 0.0,
        };
        return taffy::LayoutOutput::from_outer_size(Size {
            width: known.width.unwrap_or_else(|| fixed(style.size.width)),
            height: known.height.unwrap_or_else(|| fixed(style.size.height)),
        });
    };
    let width = match (known.width, input.available_space.width) {
        (Some(w), _) => Some(w),
        (None, AvailableSpace::Definite(w)) if m.wrap && fit => Some(w),
        _ if m.wrap => wrap_at,
        _ => None,
    };
    let s = text.shape(&m.text, m.size, m.weight, if m.wrap { width } else { None });
    let w = if m.wrap && fit { width.unwrap_or(s.width).min(s.width.max(1.0)) } else { s.width };
    taffy::LayoutOutput::from_outer_size(Size {
        width: known.width.unwrap_or(w),
        height: known.height.unwrap_or(s.height),
    })
}

impl Engine {
    fn fill(&mut self, root: &Node) -> NodeId {
        self.taffy.clear();
        build(&mut self.taffy, root)
    }

    /// Lay out `root` within `avail` (width, height) with its top-left at
    /// `origin`. Returns one rectangle per node, in pre-order.
    pub fn layout(&mut self, root: &Node, avail: (f32, f32), origin: (f32, f32), text: &mut Text) -> Vec<Rect> {
        let root_id = self.fill(root);
        self.taffy
            .compute_layout_with_measure(
                root_id,
                Size { width: AvailableSpace::Definite(avail.0), height: AvailableSpace::Definite(avail.1) },
                |input, _id, ctx, style| measure(text, input, ctx, style, None, true),
            )
            .unwrap();
        let mut out = Vec::with_capacity(self.taffy.total_node_count());
        collect(&self.taffy, root_id, origin, &mut out);
        out
    }

    /// Measure a tree's natural size without constraints (anchored labels,
    /// tooltips, cursor labels).
    pub fn natural_size(&mut self, root: &Node, max: (f32, f32), text: &mut Text) -> (f32, f32) {
        let root_id = self.fill(root);
        self.taffy
            .compute_layout_with_measure(
                root_id,
                Size { width: AvailableSpace::MaxContent, height: AvailableSpace::MaxContent },
                |input, _id, ctx, style| measure(text, input, ctx, style, Some(max.0), false),
            )
            .unwrap();
        let l = self.taffy.layout(root_id).unwrap();
        (l.size.width.min(max.0), l.size.height.min(max.1))
    }

    /// How tall a scroll area's content is when the area is `size`, from
    /// one layout of the area: the bottom of taffy's scrollable overflow
    /// rectangle (measured from the top of the padding box).
    pub fn content_height(&mut self, node: &Node, size: (f32, f32), text: &mut Text) -> f32 {
        let root_id = self.fill(node);
        let mut style = self.taffy.style(root_id).unwrap().clone();
        style.size = Size { width: Dimension::length(size.0), height: Dimension::length(size.1) };
        self.taffy.set_style(root_id, style).unwrap();
        self.taffy
            .compute_layout_with_measure(
                root_id,
                Size { width: AvailableSpace::Definite(size.0), height: AvailableSpace::Definite(size.1) },
                |input, _id, ctx, style| measure(text, input, ctx, style, None, true),
            )
            .unwrap();
        let l = self.taffy.layout(root_id).unwrap();
        l.scrollable_overflow_rect.bottom.max(l.size.height)
    }
}

fn collect(taffy: &TaffyTree<Option<Measure>>, id: NodeId, origin: (f32, f32), out: &mut Vec<Rect>) {
    let l = taffy.layout(id).unwrap();
    let (x, y) = (origin.0 + l.location.x, origin.1 + l.location.y);
    out.push([x, y, l.size.width, l.size.height]);
    for c in taffy.children(id).unwrap() {
        collect(taffy, c, (x, y), out);
    }
}
