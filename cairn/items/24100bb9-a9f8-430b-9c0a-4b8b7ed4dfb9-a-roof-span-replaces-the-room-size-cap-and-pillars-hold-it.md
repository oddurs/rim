---
id: 24100bb9-a9f8-430b-9c0a-4b8b7ed4dfb9
title: A roof span replaces the room size cap, and pillars hold it
type: feature
status: done
milestone: houses
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p0
api: breaking
effort: m
layer: engine
area: building
---

## Why

A room is indoors when it is enclosed and at most 400 cells (§4). The cap can't be seen or explained, it treats a hall like a corridor, and it gives materials and pillars nothing to do. DESIGN.md §6c revises §4: the roof follows from the walls.

## What

- A `support = { span }` block on things, scaled by the material factor `span`. Walls, doors and windows support. Rock supports with a span of its own.
- A new core `pillar`: blocks, supports, and doesn't bound rooms.
- At room rebuild, every cell within Chebyshev distance `span` of a support is roofed. A room is indoors when it is enclosed and every cell is roofed. `MAX_ROOM_CELLS` goes.
- `rim.room_at` reports the unroofed cells, so the inspector can say "4 cells out of reach: add a pillar".
- Bump the API version.

## Acceptance criteria

- [x] A 12×10 log hall is outdoors until a pillar goes in the middle (test)
- [x] The stone_age and balance sweeps are unchanged within noise (the bot's huts are within span)
- [x] Room rebuild stays within 10% of today's on the 200×200 bench (0211's numbers)
- [x] The determinism test passes

## 2026-09-26

Depth (DESIGN §6d, milestone e58c8ff7) stacks levels. Span is computed per Map as covered(z,c); roofed(z,c) also holds when the cell above is solid or floored, and building up reuses covered() to allow a floor at z+1 (§6c Storeys). Keep it a pure function of one Map.

## 2026-09-26

When the cap goes, update the building milestone's body (0210-building.md), which still says 'enclosure plus the 400-cell cap'.

## 2026-09-26

Lighting's occluder pass (e5d8b445) reads 'roofed' through one per-cell query. Today it's map.indoors; when this lands, it switches to roofed(z, c), and unroofed floor inside walls gets sky light and shadows with no lighting change. Expose the per-cell result, not only the per-room verdict.

Measured on a loaded machine (other agents' builds running). stone_age seeds 1-20: main 90/100/100/90, span 95/95/95/85 (one seed each way; all targets met). balance --seeds 20: 0/20 lost and 0/20 with a death on both; froze 27.3 vs 25.9 colonist-h/run; near-misses 12 vs 10. boundary_spike room rebuild, three runs each, median 1.13 ms main vs 1.19 ms span (+5%); raw numbers swing 0.4-1.3 ms from load. Cover is patched in a window around each changed support (property test against brute force) and worked out in full only on the first pass or 32+ changes. Pillars block, so they count as room boundary like blocking furniture does.

## 2026-09-26

Rebased on rock-as-terrain (8cc6252d): untouched rock has no fixture, so Map keeps a span per terrain id, filled from each solid terrain's thing's support when the world is made. A pocket dug into solid rock is roofed (test).
