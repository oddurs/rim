---
id: 3dd22b0d-a723-4acb-ae23-4095dc784e7e
title: 'Looks: draw primitives and sprite keys replace named shapes'
type: feature
status: backlog
milestone: graphics
depends_on:
- 4376f91e-f671-4ca5-9a6a-8c42848043eb
created: 2026-09-24
updated: 2026-09-24
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

- [ ] No match on a content name remains in the renderer
- [ ] Core plays with looks only, screenshots reviewed
- [ ] The example plugin ships an atlas and a def that uses a sprite key
- [ ] A def with no look draws as a fill in its colour, so nothing is invisible

## 2026-09-24

Graphics milestone: 'The example plugin ships an atlas and a def that uses a sprite key' now belongs to e2ce89c3 (the shared world atlas); this item defines sprite keys and validates them.
