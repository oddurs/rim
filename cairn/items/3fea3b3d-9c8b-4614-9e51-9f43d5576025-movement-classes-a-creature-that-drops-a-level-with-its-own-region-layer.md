---
id: 3fea3b3d-9c8b-4614-9e51-9f43d5576025
title: 'Movement classes: a creature that drops a level, with its own region layer'
type: feature
status: done
milestone: depth
assignee: Oddur Sigurdsson
depends_on:
- ba8253df-9f73-4196-87ac-2924e3143627
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
priority: p2
api: additive
effort: m
layer: engine
area: pathing
---

## Why

Split from ba8253df (pits and bridges): trenches and bridges stand on their own, and a second region layer per movement class is an engine change of its own size (DESIGN.md §6d).

## What

- A `[[movement]]` def (`drop = 1`) whose region layer treats a one-level drop as passable. Region layers go from per faction to per faction and class.
- Core ships no climbers; the class exists for mods.

## Acceptance criteria

- [x] A fixture mod's climber crosses a one-level drop; the extra flood fill's cost is recorded here
- [x] Determinism test passes

## 2026-09-28

Built: [[movement]] (drop 0 or 1), creature `movement`; the map keeps a climber reach table per faction, the walkers' regions joined across pit sides as well as portals, only when a loaded creature climbs; A* adds pit edges for climbers (CLIMB_COST 300, three open cells). No extra flood fill: measured on a 250 x 250 map with 976 pit cells (two double rings), join_reach costs 0.064 ms with climbers, 0 without, and runs only when regions change. The test mod's goat crosses a trench a walker can't.
