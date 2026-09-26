---
id: fa0de3f5-7ee7-4316-9e00-7db21bc43412
title: 'Wind shelter: the lee of walls, rock and trees'
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
depends_on:
- 3bb54ba3-21f9-42a4-9f7c-8299b5db1db5
created: 2026-09-23
updated: 2026-09-25
closed_at: 2026-09-25
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

- [x] Cells behind a wall are sheltered in the right direction and fade with distance (tests for all 8 directions)
- [x] Recomputed only on an octant change or a blocker change (test counts recomputes)
- [x] Full recompute at 250×250 under 0.5 ms, recorded here
- [x] The overlay shows exposure

## 2026-09-23

Moved to Colony. Rescoped: wind shelter has one user (feels-like, 0189), so it starts as a plain engine function over walls and wind direction, not a generic field kind (rule of three, DESIGN.md §6).

## 2026-09-25

Built as a field kind rather than a free function, so the overlay, hover, scripts and the next item's derived feels-like all read it with no new API: [[field]] kind = "shelter" with from = the wind direction field and lee = cells. Things declare blocks_wind (core: wall, door, window, granite 1.0; oak 0.5; berry bush 0.2); built fixtures count, blueprints don't. crates/rim_sim/src/shelter.rs: each blocker casts a lee of lee cells downwind (8 octants of the direction the wind blows toward), fading linearly; the stronger lee wins where they overlap; a blocker whose downwind neighbour blocks at least as much is skipped (the inside of a wall or a rock mass). Enclosed rooms read 0. World::update_shelter recomputes a shelter field only when (octant, map.revision) changes, never per tick; it runs after the fields each tick and once on build and restore; it's derived, never saved. Measured: full recompute at 250x250 with 6,149 fixtures, 0.403 ms (release, reference machine). Tests: all 8 directions shelter the lee and fade and leave upwind open; a half blocker casts half the lee; recomputes counted (none without change, none within an octant, one on a turn, one for a new wall); an enclosed room reads 0. Client autotest shows the overlay (18_overlay_core_wind_exposure.png).
