---
id: 642
uid: 7bd26c70-d4a1-4a2a-a1ac-dcdc451a50f4
title: Draw calls under 60 on the whole map
type: perf
status: doing
milestone: bare-metal
assignee: silver-field
created: 2026-09-28
updated: 2026-09-29
priority: p0
api: none
effort: l
layer: client
area: render
---

## Why

The whole map costs 129 to 139 draw calls and about 1.3 million indices a frame. Every call is fixed overhead on integrated GPUs and in GL submit.

## What

- Build on #340's batched pawns: batch overlays, UI and things by material and texture, and drop draws of what can't be seen.
- Report calls and indices for each pass in the bench.

## Acceptance criteria

- [x] The whole map runs at 60 draw calls or fewer (bench)
- [ ] No view's world + ui time gets worse on the same runner class

## 2026-09-29

PAUSED (2026-09-29). Done: #390 (chunk meshes joined into 2x2 regions; whole map 130 to 59 calls, same indices), ready in the queue. Left: tick criterion 1 (59 calls) and criterion 2 (world + ui on a same-class CI A/B, and the zooming view's mesh_worst_ms) once rim-c2's A/B is in, then close. Next in the stream: indices, 1.25M on the whole map, by a far-zoom level of detail for chunk paint (measure the per-layer and per-kind split first). Also owed to lucky-harbor: mesh.rs's zoom repaint budget is wall-clock (ZOOM_BUDGET_US), so which chunks repaint depends on machine speed and some autotest shots differ run to run; make it a chunk count per frame. Not yet filed as an item, because of the freeze.

## 2026-09-29

Merged as #390 (17e10f0b). Calls on CI: whole map 130 to 59, mid 91 to 55, close 44 to 39, stacked 138 to 64, below 71 to 25, dusk 140 to 65; indices unchanged. The merge A/B (36520014140 against 36520016760) crossed CPUs (EPYC 7763 against Xeon 8573C), so mid's things 0.59 to 1.04 ms was the Xeon. On EPYC 7763 alone, main just before #390 (36520014140) against three later mains (36528276891, 36529892384, 36530055937), world ms: whole map 0.613 to 0.652, mid 0.754 to 0.789, close 0.619 to 0.624, zooming 1.384 to 1.311, stacked 0.785 to 0.782; but dusk 0.756 to 0.927 and below 0.423 to 0.509, from the things pass. The later mains carry other merges, so a clean pair (17e10f0b^ against 17e10f0b on one CPU) decides criterion 2.
