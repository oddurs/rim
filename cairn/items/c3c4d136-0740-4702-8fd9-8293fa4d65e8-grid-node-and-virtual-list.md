---
id: c3c4d136-0740-4702-8fd9-8293fa4d65e8
title: Grid node and virtual list
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
priority: p0
api: additive
effort: m
layer: engine
area: ui
---

## Why

The Work Board is 30 colonists by 12 work types. As boxes that is 360 nodes
in a tree the 1 ms budget was measured on at 349. A grid has to be one node
the engine positions, and a long list has to build only what is on screen.

## What

- `kind = "grid"`: `rows`, `cols`, a `cell(r, c)` function returning text,
  colour and an optional tag, laid out as one node; hit-test to a cell;
  a drag across cells reports `on_paint(r, c, value)` once per changed
  cell, with the value the drag started with.
- `kind = "list"` (virtual): `count` and `row(i)`; only the visible span
  plus a margin is built, scrolling keeps the span current.
- Kit: `kit.grid`, `kit.table` (a virtual list with a header row).

## Acceptance criteria

- [x] A 30x12 grid adds one node to the tree, not 360
- [x] Dragging across cells paints each once, in order, with the start value
- [x] A 200-row list builds under 40 rows and scrolls to the last one
- [x] The whole-UI budget test passes with a 30x12 grid mounted

## 2026-09-24

Grid is one leaf: cells are read by calling cell(r, c) at build and painted, not laid out; its size is rows x cols of cell size so taffy never sees a cell. Drag-paint keeps a set of cells painted this press, so wandering back over one is silent. The list is a scroll node whose children are a top spacer, the rows in the visible span plus four of overscan each side, and a bottom spacer; the span comes from last frame's scroll offset and rect, so it lags one rebuild behind a fast wheel, and always keeps the last row so the content never measures as empty. A childless root sized only by its own w/h laid out at zero before (taffy passes it nothing known); measure now falls back to its fixed size.
