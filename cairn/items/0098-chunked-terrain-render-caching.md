---
id: e3c1f87b-12e7-4112-ad0e-028f9284e86a
title: 'Chunked meshes: what does not move is drawn from the GPU'
type: perf
status: doing
milestone: graphics
assignee: Oddur Sigurdsson
claimed: 2026-09-24
depends_on:
- 96d2dac9-cf4e-409a-a028-f49902c8d7d9
- 3dd22b0d-a723-4acb-ae23-4095dc784e7e
- e2ce89c3-de39-43ac-a302-d2dcc325aedf
created: 2026-09-22
updated: 2026-09-24
priority: p0
api: none
effort: m
layer: client
area: render
pillar:
- performance
---

## Why

Every frame, `draw::world` visits three layers of every visible cell, looks
up each entity's components, and rebuilds the vertices of every floor,
wall, tree and item from nothing. Almost none of it changed since the last
frame. On a weak CPU that loop is the frame.

## What

- One vertex buffer per chunk per layer (floors, items, fixtures), built
  from looks and the world atlas, and rebuilt only when the chunk is dirty.
- A visible chunk is one draw call. Pawns, blueprint progress, fire
  flicker, hit flashes and tool previews stay immediate.
- Dirty comes from the chunk grid (the Chunks item), including changes
  that don't move anything but change how it looks: a stack's count, a
  designation, a plant picked clean, a blueprint's materials.

## Budget

World renderer fully zoomed out on the bench map ≤ 4 ms CPU at reference
(per the render bench), and flat when nothing changes.

## Measurement (before)

`rim --bench-render --seed 1`, Apple M-series, 1920×1048 points at 2x, ms
of world CPU per frame (before: looks, drawn every frame; after: chunk
meshes):

| view | before | after | after p99 | draw calls | rebuilt |
|---|---|---|---|---|---|
| whole map (z 4) | 4.23 | 0.57 | 1.47 | 49 → 98 | 0 |
| mid (z 12) | 2.56 | 0.31 | 0.87 | 32 → 60 | 0 |
| close (z 28) | 0.82 | 0.21 | 0.27 | 14 → 24 | 0 |
| storm (z 4) | 4.37 | 0.99 | 1.64 | 50 → 99 | 0 |
| zooming (new) | — | 0.47 | 3.50 | 98 | 192 in 240 frames |

Submit falls with it (0.61 → 0.39 on the whole map): the batches macroquad
builds and uploads each frame are now the ground, plans, pawns, weather and
UI. Indices drawn are the same 1.1M; they come from GPU memory. Draw calls
double (a chunk per layer), which is cheap next to the vertices they replace.
CI numbers: see the PR.


## Acceptance criteria

- [x] A chunk's buffers rebuild only when that chunk is dirty
- [x] Nothing drawn changes: autotest screenshots match before and after
- [x] Bench before and after recorded here

## 2026-09-24

Graphics milestone: no longer waits on the ground shader (da558444, still in Scale), which is about drawing wetness and snow; caching things per chunk doesn't need it. Retitled from 'Chunked terrain render caching': terrain is already one texture.

## 2026-09-24

When this lands, drop the render bench's CI slack from 3x toward 1x (bench.rs, --check): the gate only holds the 4 ms budget once things are cached.

## 2026-09-24

Buffers are raw miniquad (Immutable vertex/index buffers, a GLSL 100 pipeline), not macroquad's draw_mesh, which would copy and re-upload every vertex each frame. Painting goes through a Sink shared with the immediate path, so the two can't drift. Line widths and min sizes are in points, so a chunk also depends on the zoom: while it moves, chunks draw scaled from the zoom they were built at and rebuild once it holds for 8 frames (or drifts past 2x), within 1.5 ms a frame. Plans and animated looks (pulse) are drawn live each frame. Chunk-border draw order differs from row order, which can only matter where an overhang crosses a chunk edge; screenshots show no difference. CI slack drops from 3x to 1.5x.
