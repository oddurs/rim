---
id: 06014a48-fb9e-4825-9390-664f32ee2d45
title: 'Lay out incrementally: keep taffy''s tree between frames and relayout only what changed'
type: perf
status: backlog
milestone: interface
created: 2026-09-26
updated: 2026-09-26
priority: p2
api: none
effort: m
layer: client
area: ui
---

## Budget

## Measurement (before)

## Approach

## Acceptance criteria

- [ ] Benchmark shows the budget is met

The docked UI is one layout tree, rebuilt from scratch (`Engine::fill`
clears taffy) and recomputed whenever any docked tree changes. With hover
rebuilds narrowed to the trees that read it (28a49214), the pointer
crossing a cell still costs ~1.25 ms of CPU a frame over holding still,
almost all of it relaying out ~400 nodes because the hover readout's
text changed. Taffy caches per node when the tree persists: diff the
built tree against the last by node key, update styles and measure
contexts in place, mark dirty only what changed.

Tried and not enough: a shape-cache lookup without allocating made no
measurable difference.

## Acceptance criteria

- [ ] A change inside one docked tree relays out that tree and its ancestors, not the rest
- [ ] Crossing a cell costs within 0.2 ms of holding still on the core UI with 30 colonists
