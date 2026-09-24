---
id: 4376f91e-f671-4ca5-9a6a-8c42848043eb
title: Render benchmark and a frame budget CI holds
type: perf
status: doing
milestone: graphics
assignee: Oddur Sigurdsson
claimed: 2026-09-24
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

`rim --bench-render --seed 1`, 300 frames per view, ms per frame. World is
CPU for every pass but the UI; submit is macroquad handing the batches to
GL; gpu is glFinish (Linux only).

GitHub ubuntu-latest, Xvfb + llvmpipe (software GL), 1600×960 at 1x:

| view | world | p99 | things | pawns | weather | ui | submit | gpu | calls | indices |
|---|---|---|---|---|---|---|---|---|---|---|
| whole map (z 4) | 6.03 | 7.12 | 5.84 | 0.19 | 0.00 | 0.14 | 24.8 | 37.6 | 29 | 630k |
| mid (z 12) | 2.29 | 3.78 | 2.18 | 0.11 | 0.00 | 0.12 | 8.1 | 71.6 | 12 | 227k |
| close (z 28) | 0.75 | 1.07 | 0.66 | 0.09 | 0.00 | 0.42 | 4.2 | 69.9 | 7 | 94k |
| storm (z 4) | 6.30 | 6.59 | 5.79 | 0.18 | 0.33 | 0.11 | 34.5 | 57.2 | 30 | 649k |

Apple M-series, 1600×960 points at 2x: whole map 3.14 world (3.07
things), mid 2.11, close 0.77, storm 3.47; submit ~0.4.

Things are the frame: 97% of the world's CPU fully zoomed out, and over
budget on the CI runner. The CI gate runs at 2x slack until the chunked
meshes land; that item tightens it.


## Acceptance criteria

- [x] `rim --bench-render` prints per-pass time, vertices and draw calls at three zooms
- [x] CI runs it with `--check` and fails over budget
- [x] Baseline numbers recorded in this item
- [x] DESIGN.md §8 names the reference machine and the render budget

## 2026-09-24

Per-pass times also show in the F3 profiler as draw:* rows. Draw calls come from macroquad's frame capture on one frame per view (it re-issues each call into a small texture, so that frame isn't timed). --shots DIR writes one screenshot per view, which later items diff against.
