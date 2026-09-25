---
id: 4376f91e-f671-4ca5-9a6a-8c42848043eb
title: Render benchmark and a frame budget CI holds
type: perf
status: done
milestone: graphics
assignee: Oddur Sigurdsson
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
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
GL; gpu is glFinish (Linux only). The colony's buildings are owned, so
deconstruct marks every one of them: the worst case for markers.

GitHub ubuntu-latest, Xvfb + llvmpipe (software GL), 1920×1080 at 1x:

| view | world | p99 | things | pawns | weather | ui | submit | gpu | calls | indices |
|---|---|---|---|---|---|---|---|---|---|---|
| whole map (z 4) | 5.68 | 6.48 | 5.51 | 0.16 | 0.01 | 0.08 | 30.0 | 27.3 | 49 | 1.11M |
| mid (z 12) | 3.55 | 3.91 | 3.42 | 0.11 | 0.01 | 0.08 | 18.9 | 56.7 | 32 | 699k |
| close (z 28) | 1.26 | 2.01 | 1.19 | 0.06 | 0.01 | 0.30 | 10.7 | 62.8 | 14 | 247k |
| storm (z 4) | 5.66 | 9.81 | 5.26 | 0.15 | 0.25 | 0.07 | 35.1 | 39.9 | 50 | 1.13M |

Apple M-series, 1920×1048 points at 2x: whole map 3.90 world (3.85
things), mid 3.15, close 1.75, storm 4.02; submit ~0.5.

Things are the frame: 97% of the world's CPU fully zoomed out, and over
budget on both machines. Submit is the second cost: 1.1M indices built
and uploaded every frame. The CI gate runs at 3x slack until the chunked
meshes land; that item tightens it.

## Acceptance criteria

- [x] `rim --bench-render` prints per-pass time, vertices and draw calls at three zooms
- [x] CI runs it with `--check` and fails over budget
- [x] Baseline numbers recorded in this item
- [x] DESIGN.md §8 names the reference machine and the render budget

## 2026-09-24

Per-pass times also show in the F3 profiler as draw:* rows. Draw calls come from macroquad's frame capture on one frame per view (it re-issues each call into a small texture, so that frame isn't timed). --shots DIR writes one screenshot per view, which later items diff against.
