---
id: 3dd22b0d-a723-4acb-ae23-4095dc784e7e
title: 'Looks: draw primitives and sprite keys replace named shapes'
type: feature
status: done
milestone: graphics
assignee: Oddur Sigurdsson
depends_on:
- 4376f91e-f671-4ca5-9a6a-8c42848043eb
created: 2026-09-24
updated: 2026-09-25
closed_at: 2026-09-25
priority: p0
api: breaking
effort: l
layer: engine
area: render
pillar:
- plugin-first
---

## Why

`Shape` names content: fourteen variants from `tree` to `stove`, and the
renderer decides what joins to a wall by matching on `wall`, `window` and
`door` (`joins_wall` in `draw.rs`). A mod that adds a loom picks the least
wrong of those, a fence cannot join, and a mod that ships a tileset cannot
use it at all. DESIGN.md §6a rules that core is plain and the look is data:
the renderer reads a def's look and never matches on what the thing is.

Breaking (`shape` goes away), so it lands before the save format, per §10.

## What

- `look` on a def: a colour plus one primitive from a small fixed set
  (fill, outline, disc, glyph) or a sprite key into an atlas the mod ships.
- Primitives stay in code; the named content shapes are deleted from
  `defs.rs` and `draw.rs`.
- Joining is a data rule on the look: which neighbours count as joined and
  which variant each neighbour mask picks. `joins_wall` goes.
- Core's defs move to primitives and glyphs. No sprites in core.
- Mod validation: an unknown sprite key or atlas is a load-time error that
  names the mod.

## Acceptance criteria

- [x] No match on a content name remains in the renderer
- [x] Core plays with looks only, screenshots reviewed
- [x] A def with no look draws as a fill in its colour, so nothing is invisible

## 2026-09-24

Graphics milestone: 'The example plugin ships an atlas and a def that uses a sprite key' now belongs to e2ce89c3 (the shared world atlas); this item defines sprite keys and validates them.

## 2026-09-24

Layers of fill, outline, disc and edges, with shade, vary (per-cell brightness), min_px and pulse; look.join interns labels to groups at load so the renderer compares integers. No glyph primitive: text isn't in the world batch today, and a glyph belongs in the shared world atlas (e2ce89c3), so it moves there with sprite keys. A def that still has `shape` is a load error naming it; API 0.4. Screenshot diff against the bench's views before and after: only the campfire's flicker (time-based) differs; index counts are identical in every view.

## 2026-09-24

Review: padding now only on a dimension the fill spans whole (granite's bottom strip spilled half a point); colours are checked as hex before slicing (a non-ASCII colour panicked); layer numbers are range-checked. Declined: the picked bush's colour stays a fixed hex rather than derived from the def's; a fixed colour is the author's choice, and looks.md now says fixed colours don't follow a patched color.
