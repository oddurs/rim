---
id: 66906291-d86d-48d1-ba7b-877828ac5344
title: Incremental region updates
type: perf
status: backlog
milestone: scale
depends_on:
- 96d2dac9-cf4e-409a-a028-f49902c8d7d9
- ee7fe7fd-4cb1-4f65-b23c-2e325c641fdc
created: 2026-09-22
updated: 2026-09-27
priority: p2
api: none
effort: m
layer: engine
area: map
pillar:
- performance
---

## Budget

Placing a wall re-floods one chunk, not the map.

## Measurement (before)


## Acceptance criteria

- [ ] Chunk-local region rebuild

## 2026-09-24

Uses the chunk grid from 'Chunks: one dirty unit for regions, fields and the renderer' rather than its own; one dirty unit for regions, fields and the renderer (DESIGN.md §6a).

## 2026-09-27

Depth (acd85584) measured the cost this item removes. bench --levels (two dug rooms under the colony, every colonist mining underground) spends ~0.2 ms a tick in 'rooms' because each dug rock cell rebuilds rooms over all four levels (O(cells)). Regions are already per level; rooms and their ids are next.
