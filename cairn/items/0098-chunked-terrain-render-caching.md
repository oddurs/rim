---
id: e3c1f87b-12e7-4112-ad0e-028f9284e86a
title: 'Chunked meshes: what does not move is drawn from the GPU'
type: perf
status: backlog
milestone: graphics
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


## Acceptance criteria

- [ ] A chunk's buffers rebuild only when that chunk is dirty
- [ ] Nothing drawn changes: autotest screenshots match before and after
- [ ] Bench before and after recorded here

## 2026-09-24

Graphics milestone: no longer waits on the ground shader (da558444, still in Scale), which is about drawing wetness and snow; caching things per chunk doesn't need it. Retitled from 'Chunked terrain render caching': terrain is already one texture.
