---
id: 97
uid: 66906291-d86d-48d1-ba7b-877828ac5344
title: Incremental region updates
type: perf
status: backlog
milestone: scale
depends_on:
- 241
- 93
created: 2026-09-22
updated: 2026-09-28
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

## 2026-09-28

Measured 2026-09-28 with the sim bench's --scale ladder (981e6792), M4 Pro at load 91-115: regions grow with map cells at exponent 1.90 (ms per tick at 128², 250², 500², 1000² with 200 pawns: 0.0023, 0.0000, 0.3964, 0.6187; the 250² rung saw no rebuild in its 0.1 days). The whole tick at 1000² is 6.2 ms, over the 2 ms budget, and regions, rooms and shelter are 3.4 ms of it. Worth moving into bare-metal (the user's call).
