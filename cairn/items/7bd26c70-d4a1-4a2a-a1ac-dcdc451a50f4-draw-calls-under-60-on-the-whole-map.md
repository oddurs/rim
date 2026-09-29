---
id: 7bd26c70-d4a1-4a2a-a1ac-dcdc451a50f4
title: Draw calls under 60 on the whole map
type: perf
status: doing
milestone: bare-metal
assignee: silver-field
created: 2026-09-28
updated: 2026-09-28
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

## 2026-09-28

Measured before any code (a probe flushing macroquad at each render pass on the bench's capture frame, seed 1, 20 sprite mods; counts only, so the hidden window doesn't matter). Whole map: 130 calls = 117 chunk meshes + 1 figure batch + 12 macroquad calls (things 4, ui 3, and 1 each for the pawns-pass flush, the light multiply, roofs, the world-target blit and world_ui). The probe's sum equals the bench's total, so the attribution is exact. The chunk meshes also carry 1.25M of the 1.27M indices. Other views: mid 91 (78 meshes), close 46 (32), stacked 139 (122), dusk 140 (123), below 71 (64). So the draw calls are a mesh.rs problem: 64 chunks a level times 3 layers, and a part per atlas-page run in each. UI and overlays are 3 to 8 calls and not worth batching first.

## 2026-09-28

Plan. (1) Mesh vertices in map cells, not points at the build zoom, so every chunk draws with the same uniforms: the per-chunk origin and scale uniforms go, and a chunk built at another zoom is still drawn at the current one, as scale did. (2) Chunks merged per layer into regions of 2 by 2 chunks: each chunk keeps its CPU geometry and rebuilds alone, and the region's buffer is its chunks' parts concatenated, a run on one page merging across chunk borders. A region wholly on screen is one call per page run. A partly visible one draws only its visible chunks' index ranges, so close zoom draws no more indices than today. Whole map: at most 16 regions x 3 layers, fewer where empty. (3) If the per-chunk part counts show sprite pages splitting runs, solid shapes use the current page's white texel instead of page 0's. Numbers: calls and indices per view from the bench, and world + ui on two same-class CI runs.
