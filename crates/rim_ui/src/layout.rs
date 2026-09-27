//! Layout: a node tree in, one rectangle per node (pre-order) out.
//!
//! Flexbox via taffy; text leaves are measured through the shaping cache.
//! A named layout (the docked shell, each window) keeps its taffy tree
//! between frames: an unchanged tree skips layout entirely, and a changed
//! one is reconciled into the kept tree so that taffy lays out again only
//! what changed and the boxes it sits in.

use crate::node::{Align, Kind, Len, Node};
use crate::text::Text;
use std::collections::HashMap;
use taffy::prelude::*;
use taffy::style::{ExpandedDimension, ExpandedLengthPercentage};
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
#[derive(Clone, PartialEq)]
struct Measure {
    text: String,
    size: f32,
    weight: u16,
    wrap: bool,
    tracking: f32,
}

fn style_of(n: &Node) -> Style {
    let s = &n.style;
    Style {
        flex_direction: if s.row { FlexDirection::Row } else { FlexDirection::Column },
        flex_wrap: if s.wrap { FlexWrap::Wrap } else { FlexWrap::NoWrap },
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
        flex_shrink: if matches!(n.kind, Kind::Text | Kind::Grid | Kind::Image | Kind::Input) { 0.0 } else { 1.0 },
        align_items: s.align.map(align_items),
        justify_content: s.justify.map(justify),
        overflow: if n.kind == Kind::Scroll {
            Point { x: Overflow::Visible, y: Overflow::Scroll }
        } else {
            Point { x: Overflow::Visible, y: Overflow::Visible }
        },
        scrollbar_width: 0.0,
        ..Default::default()
    }
}

fn measure_of(n: &Node) -> Option<Measure> {
    n.text.as_ref().map(|t| Measure {
        text: t.text.clone(),
        size: t.size,
        weight: t.weight,
        wrap: t.wrap,
        tracking: t.tracking,
    })
}

fn build(taffy: &mut TaffyTree<Option<Measure>>, n: &Node) -> NodeId {
    if let Some(m) = measure_of(n) {
        return taffy.new_leaf_with_context(style_of(n), Some(m)).unwrap();
    }
    let kids: Vec<NodeId> = n.children.iter().map(|c| build(taffy, c)).collect();
    taffy.new_with_children(style_of(n), &kids).unwrap()
}

/// A kept taffy node and what it was built from, to tell what changed.
struct Mirror {
    id: NodeId,
    style: Style,
    text: Option<Measure>,
    children: Vec<Mirror>,
}

fn grow(taffy: &mut TaffyTree<Option<Measure>>, n: &Node) -> Mirror {
    let (style, text) = (style_of(n), measure_of(n));
    if text.is_some() {
        let id = taffy.new_leaf_with_context(style.clone(), text.clone()).unwrap();
        return Mirror { id, style, text, children: Vec::new() };
    }
    let children: Vec<Mirror> = n.children.iter().map(|c| grow(taffy, c)).collect();
    let ids: Vec<NodeId> = children.iter().map(|c| c.id).collect();
    let id = taffy.new_with_children(style.clone(), &ids).unwrap();
    Mirror { id, style, text, children }
}

fn prune(taffy: &mut TaffyTree<Option<Measure>>, m: Mirror) {
    for c in m.children {
        prune(taffy, c);
    }
    let _ = taffy.set_node_context(m.id, None);
    let _ = taffy.remove(m.id);
}

/// Bring the kept tree `m` in line with `n`, children matched by position.
/// Setting a style, a text or a child list marks that node and its
/// ancestors dirty; everything else keeps taffy's cached layout.
fn sync(taffy: &mut TaffyTree<Option<Measure>>, m: &mut Mirror, n: &Node) {
    let (style, text) = (style_of(n), measure_of(n));
    // A text leaf and a box are different kinds of taffy node.
    if text.is_some() != m.text.is_some() {
        let old = std::mem::replace(m, grow(taffy, n));
        prune(taffy, old);
        return;
    }
    if style != m.style {
        taffy.set_style(m.id, style.clone()).unwrap();
        m.style = style;
    }
    if text != m.text {
        taffy.set_node_context(m.id, Some(text.clone())).unwrap();
        m.text = text;
    }
    let before: Vec<NodeId> = m.children.iter().map(|c| c.id).collect();
    for (mc, nc) in m.children.iter_mut().zip(&n.children) {
        sync(taffy, mc, nc);
    }
    let kept = n.children.len().min(m.children.len());
    for gone in m.children.split_off(kept) {
        prune(taffy, gone);
    }
    for nc in &n.children[kept..] {
        m.children.push(grow(taffy, nc));
    }
    let after: Vec<NodeId> = m.children.iter().map(|c| c.id).collect();
    if after != before {
        taffy.set_children(m.id, &after).unwrap();
    }
}

/// A named layout as last computed.
struct Kept {
    root: Mirror,
    /// The tree's layout hash, the space and origin it was laid out in.
    key: (u64, (f32, f32), (f32, f32)),
    rects: Vec<Rect>,
    /// `Engine::frame` when last asked for.
    used: u64,
}

/// How many frames a named layout nobody asks for is kept (a closed window).
const KEEP_FRAMES: u64 = 600;

/// The layout engine. One-off layouts (labels, tooltips, measuring) share a
/// taffy tree that is cleared and refilled for each call, so its storage is
/// reused; named layouts keep theirs (`layout_kept`).
pub struct Engine {
    taffy: TaffyTree<Option<Measure>>,
    kept_taffy: TaffyTree<Option<Measure>>,
    kept: HashMap<String, Kept>,
    frame: u64,
    /// Layouts computed (not served from the kept rects), for tests and the
    /// profiler.
    pub computed: u64,
}

impl Default for Engine {
    fn default() -> Self {
        Engine { taffy: TaffyTree::new(), kept_taffy: TaffyTree::new(), kept: HashMap::new(), frame: 0, computed: 0 }
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
    // A text leaf's padding is part of its box: the text sits inside it
    // (paint offsets by the same amount), so an input has room around its
    // line and a padded label measures as a padded label.
    let pad = |lp: LengthPercentage| match lp.expand() {
        ExpandedLengthPercentage::Length(v) => v,
        _ => 0.0,
    };
    let (px, py) =
        (pad(style.padding.left) + pad(style.padding.right), pad(style.padding.top) + pad(style.padding.bottom));
    let width = match (known.width, input.available_space.width) {
        (Some(w), _) => Some(w - px),
        (None, AvailableSpace::Definite(w)) if m.wrap && fit => Some(w - px),
        _ if m.wrap => wrap_at.map(|w| w - px),
        _ => None,
    };
    let s = text.shape(&m.text, m.size, m.weight, m.tracking, if m.wrap { width } else { None });
    let w = if m.wrap && fit { width.unwrap_or(s.width).min(s.width.max(1.0)) } else { s.width };
    taffy::LayoutOutput::from_outer_size(Size {
        width: known.width.unwrap_or(w + px),
        height: known.height.unwrap_or(s.height + py),
    })
}

impl Engine {
    /// Call once per frame: named layouts unused for a while are dropped.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
        if self.frame.is_multiple_of(KEEP_FRAMES) {
            let now = self.frame;
            let stale: Vec<String> =
                self.kept.iter().filter(|(_, k)| now - k.used >= KEEP_FRAMES).map(|(n, _)| n.clone()).collect();
            for name in stale {
                if let Some(k) = self.kept.remove(&name) {
                    prune(&mut self.kept_taffy, k.root);
                }
            }
        }
    }

    /// Drop every kept layout: text metrics or the scale changed, so no
    /// measurement taffy cached still holds.
    pub fn forget(&mut self) {
        self.kept.clear();
        self.kept_taffy.clear();
    }

    /// Lay out `root` as the layout called `name`, whose layout hash is
    /// `hash`. The same hash, space and origin as last time return the last
    /// rects; otherwise the kept tree is updated and laid out again, and
    /// taffy recomputes only the nodes that changed and their ancestors.
    pub fn layout_kept(
        &mut self,
        name: &str,
        hash: u64,
        root: &Node,
        avail: (f32, f32),
        origin: (f32, f32),
        text: &mut Text,
    ) -> Vec<Rect> {
        let key = (hash, avail, origin);
        let frame = self.frame;
        let tree = &mut self.kept_taffy;
        let mirror = match self.kept.remove(name) {
            Some(k) if k.key == key => {
                let rects = k.rects.clone();
                self.kept.insert(name.to_string(), Kept { used: frame, ..k });
                return rects;
            }
            Some(mut k) => {
                sync(tree, &mut k.root, root);
                k.root
            }
            None => grow(tree, root),
        };
        self.computed += 1;
        tree.compute_layout_with_measure(
            mirror.id,
            Size { width: AvailableSpace::Definite(avail.0), height: AvailableSpace::Definite(avail.1) },
            |input, _id, ctx, style| measure(text, input, ctx, style, None, true),
        )
        .unwrap();
        let mut rects = Vec::new();
        collect(tree, mirror.id, origin, &mut rects);
        self.kept.insert(name.to_string(), Kept { root: mirror, key, rects: rects.clone(), used: frame });
        rects
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Style as S, TextStyle};
    use crate::plain;

    fn label(key: u64, s: &str, wrap: bool) -> Node {
        let mut n = plain(key, S { pad: [2.0, 4.0, 2.0, 4.0], ..Default::default() }, vec![]);
        n.kind = Kind::Text;
        n.text = Some(TextStyle { text: s.into(), size: 14.0, weight: 400, color: [1.0; 4], wrap, tracking: 0.0 });
        n
    }

    fn column(key: u64, kids: Vec<Node>) -> Node {
        plain(key, S { gap: 6.0, pad: [8.0; 4], ..Default::default() }, kids)
    }

    /// A row `w` wide of `n` words that wraps them in lines, or doesn't.
    fn flow(key: u64, n: u64, w: f32, wrap: bool) -> Node {
        let words = (0..n).map(|i| label(key + 1 + i, &format!("word {i}"), false)).collect();
        plain(key, S { row: true, wrap, gap: 4.0, w: Len::Px(w), ..Default::default() }, words)
    }

    /// A shell like the docked layer: a bar on top, then a band of left
    /// column, growing centre and right column.
    fn shell(left: Vec<Node>, right: Vec<Node>, gap: f32) -> Node {
        let top = plain(10, S { row: true, w: Len::Frac(1.0), ..Default::default() }, vec![label(11, "Day 1", false)]);
        let band = plain(
            20,
            S {
                row: true,
                gap,
                grow: 1.0,
                w: Len::Frac(1.0),
                min_h: Some(0.0),
                align: Some(Align::Stretch),
                ..Default::default()
            },
            vec![column(30, left), plain(40, S { grow: 1.0, ..Default::default() }, vec![]), column(50, right)],
        );
        plain(1, S { w: Len::Px(1600.0), h: Len::Px(960.0), ..Default::default() }, vec![top, band])
    }

    /// Every edit the kept tree takes must lay out exactly as the same tree
    /// built from nothing: new text, a new or lost child, a changed style,
    /// a text leaf turned box and back.
    #[test]
    fn a_kept_tree_lays_out_as_one_built_fresh() {
        let mut text = Text::new(None, &[]).expect("a font");
        let mut kept = Engine::default();
        let mut fresh = Engine::default();
        let steps: Vec<Node> = vec![
            shell(vec![label(31, "Colonists", false)], vec![label(51, "Dirt 1, 2", false)], 4.0),
            shell(vec![label(31, "Colonists", false)], vec![label(51, "Rich soil 112, 115", false)], 4.0),
            shell(
                vec![label(31, "Colonists", false), label(32, "Gunnar, the founder, wakes with nothing.", true)],
                vec![label(51, "Rich soil 112, 115", false)],
                4.0,
            ),
            shell(vec![label(32, "Gunnar", false)], vec![label(51, "Dirt", false), label(52, "outdoors", false)], 4.0),
            shell(vec![label(32, "Gunnar", false)], vec![label(51, "Dirt", false), label(52, "outdoors", false)], 12.0),
            shell(vec![column(33, vec![label(34, "a box now", false)])], vec![label(51, "Dirt", false)], 12.0),
            shell(vec![label(33, "a leaf again", false)], vec![], 12.0),
            // A row that wraps, then stops wrapping, then wraps again.
            shell(vec![flow(60, 12, 300.0, true)], vec![], 12.0),
            shell(vec![flow(60, 12, 300.0, false)], vec![], 12.0),
            shell(vec![flow(60, 14, 300.0, true)], vec![], 12.0),
        ];
        for (i, tree) in steps.iter().enumerate() {
            let got = kept.layout_kept("docked", i as u64, tree, (1600.0, 960.0), (0.0, 0.0), &mut text);
            let want = fresh.layout(tree, (1600.0, 960.0), (0.0, 0.0), &mut text);
            assert_eq!(got, want, "step {i}");
        }
        assert_eq!(kept.computed, steps.len() as u64, "every step changed the tree");
        let last = steps.len() - 1;
        let again = kept.layout_kept("docked", last as u64, &steps[last], (1600.0, 960.0), (0.0, 0.0), &mut text);
        assert_eq!(kept.computed, steps.len() as u64, "the same tree again is served, not laid out");
        assert_eq!(again, fresh.layout(&steps[last], (1600.0, 960.0), (0.0, 0.0), &mut text));
    }

    /// Twenty words in a row 300 wide: wrapped, every one inside the row,
    /// in lines `gap` apart; unwrapped, they run past its edge.
    #[test]
    fn a_wrapping_row_lays_its_children_in_lines() {
        let mut text = Text::new(None, &[]).expect("a font");
        let mut e = Engine::default();
        let rects = e.layout(&flow(100, 20, 300.0, true), (1600.0, 960.0), (0.0, 0.0), &mut text);
        let words = &rects[1..];
        assert_eq!(words.len(), 20);
        assert!(words.iter().all(|r| r[0] >= 0.0 && r[0] + r[2] <= 300.0 + 0.01), "all inside: {words:?}");
        let mut lines: Vec<f32> = words.iter().map(|r| r[1]).collect();
        lines.dedup();
        assert!(lines.len() >= 3, "twenty words take several lines: {lines:?}");
        assert!((lines[1] - lines[0] - (words[0][3] + 4.0)).abs() < 0.01, "a gap between lines");
        assert!(rects[0][3] >= lines.len() as f32 * words[0][3], "the row is as tall as its lines");

        let rects = e.layout(&flow(100, 20, 300.0, false), (1600.0, 960.0), (0.0, 0.0), &mut text);
        assert!(rects[1..].iter().all(|r| r[1] == rects[1][1]), "without wrap, one line");
    }
}
