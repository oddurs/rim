---
id: 642
uid: 7bd26c70-d4a1-4a2a-a1ac-dcdc451a50f4
title: Draw calls under 60 on the whole map
type: perf
status: backlog
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

- [ ] The whole map runs at 60 draw calls or fewer (bench)
- [ ] No view's world + ui time gets worse on the same runner class

## 2026-09-29

PAUSED (2026-09-29). Done: #390 (chunk meshes joined into 2x2 regions; whole map 130 to 59 calls, same indices), ready in the queue. Left: tick criterion 1 (59 calls) and criterion 2 (world + ui on a same-class CI A/B, and the zooming view's mesh_worst_ms) once rim-c2's A/B is in, then close. Next in the stream: indices, 1.25M on the whole map, by a far-zoom level of detail for chunk paint (measure the per-layer and per-kind split first). Also owed to lucky-harbor: mesh.rs's zoom repaint budget is wall-clock (ZOOM_BUDGET_US), so which chunks repaint depends on machine speed and some autotest shots differ run to run; make it a chunk count per frame. Not yet filed as an item, because of the freeze.
