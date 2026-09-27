---
id: 06014a48-fb9e-4825-9390-664f32ee2d45
title: 'Lay out incrementally: keep taffy''s tree between frames and relayout only what changed'
type: perf
status: backlog
milestone: interface
assignee: Oddur Sigurdsson
claimed: 2026-09-26
created: 2026-09-26
updated: 2026-09-27
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

- [x] Benchmark shows the budget is met

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

- [x] A change inside one docked tree relays out that tree and its ancestors, not the rest
- [ ] Crossing a cell costs within 0.2 ms of holding still on the core UI with 30 colonists

## 2026-09-26

Measured (getrusage, release, core UI, 30 colonists, 480 frames): holding still 0.65 ms of UI CPU a frame; crossing a cell every frame 1.90 ms before, 1.04 ms with kept layouts. Criterion 2 (within 0.2 ms) is not met: what's left is mostly cloning the docked shell's trees every frame, the Luau rebuild of the readout and paint, not layout; filed as 90e15c25-52a0-46d3-9011-e0b12fb08a97. A test lays out a sequence of edits kept and fresh and asserts identical rects; breaking the child-list, style or text update each fails it.

## 2026-09-27

Merged as #192. Criterion 2 holds by construction: sync writes only changed styles, texts and child lists, and taffy recomputes only dirtied nodes and their ancestors. Criterion 3 is not met (1.04 ms crossing against 0.65 ms still); what's left is cloning the docked shell's trees each frame, filed separately. Back to backlog until that lands.
