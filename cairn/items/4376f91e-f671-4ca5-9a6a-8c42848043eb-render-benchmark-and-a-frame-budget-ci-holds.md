---
id: 4376f91e-f671-4ca5-9a6a-8c42848043eb
title: Render benchmark and a frame budget CI holds
type: perf
status: backlog
milestone: graphics
created: 2026-09-24
updated: 2026-09-24
priority: p0
api: none
effort: m
layer: client
area: render
---

## Why

Nothing measures the renderer. §8 gives the sim 2 ms a tick and the UI 1 ms
a frame, and the world draw gets "60 fps on a mid-range laptop". Every other
item in this milestone claims a speedup, and without a number none of them
can show one.

## What

- `rim --bench-render`: the §8 world (250×250, 30 colonists, 200 pawns) with
  a dense colony stamped in (walls, floors, furniture, stacked items) and
  forest elsewhere, drawn for N frames at three zooms (fully out, mid,
  close) plus weather at full precipitation.
- Reports CPU time per pass (things, pawns, weather, light, UI), vertices
  and draw calls per frame, as a table and as JSON.
- `--check` fails over budget, with slack for shared runners like the
  sim benchmark. Runs in CI under Xvfb (software GL, a fair stand-in for a
  weak GPU).
- DESIGN.md §8 gains the reference machine and the world renderer's budget.

## Budget

World renderer ≤ 4 ms CPU a frame on the reference machine, fully zoomed
out.

## Measurement (before)


## Acceptance criteria

- [ ] `rim --bench-render` prints per-pass time, vertices and draw calls at three zooms
- [ ] CI runs it with `--check` and fails over budget
- [ ] Baseline numbers recorded in this item
- [ ] DESIGN.md §8 names the reference machine and the render budget
