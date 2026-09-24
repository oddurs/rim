---
id: 188
title: 'Wind shelter: the lee of walls, rock and trees'
type: feature
status: backlog
milestone: colony
depends_on:
- 183
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: additive
effort: m
layer: engine
area: map
pillar:
- survival
- performance
---

## Why

A wall on the windward side should matter even without a roof: it keeps the wind off, the snow off and the campfire lit. Wind shelter makes building layout respond to the weather.

## What

- Thing defs gain `blocks_wind` (0–1; walls and rock 1.0, trees 0.5).
- `wind_exposure` grid: from each blocker, cast downwind over the current 8-way wind direction for `lee_length` cells, with shelter fading along the lee.
- Recomputed only when the wind turns into another octant or blockers change (dirty rows), never per tick. Readable as a derived field named `wind_exposure`.
- Enclosed rooms are 0.

## Acceptance criteria

- [ ] Cells behind a wall are sheltered in the right direction and fade with distance (tests for all 8 directions)
- [ ] Recomputed only on an octant change or a blocker change (test counts recomputes)
- [ ] Full recompute at 250×250 under 0.5 ms, recorded here
- [ ] The overlay shows exposure

## 2026-09-23

Moved to Colony. Rescoped: wind shelter has one user (feels-like, 0189), so it starts as a plain engine function over walls and wind direction, not a generic field kind (rule of three, DESIGN.md §6).
