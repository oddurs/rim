---
id: 98
title: Chunked terrain render caching
type: perf
status: backlog
milestone: scale
depends_on:
- 193
created: 2026-09-22
updated: 2026-09-23
priority: p1
api: none
effort: m
layer: client
area: render
pillar:
- performance
---

## Budget

Terrain draws from cached chunk textures.

## Measurement (before)


## Acceptance criteria

- [ ] Chunks redraw only when their revision changes

## 2026-09-23

The Weather sprint's ground renderer (0193) draws terrain, wetness and snow in one shader pass from map-sized textures, which removes the per-cell rectangles this item was going to cache. What's left here: chunking the textures for maps beyond 250×250 and caching things (plants, walls) by chunk.
