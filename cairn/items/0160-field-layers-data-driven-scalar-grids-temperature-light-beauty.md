---
id: 6adcd8f6-07a8-44d2-9135-872bcc0115ad
title: 'Field layers: data-driven scalar grids (temperature, light, beauty)'
type: feature
status: done
milestone: shelter
assignee: Oddur Sigurdsson
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
priority: p0
api: additive
effort: l
layer: engine
area: sim
pillar:
- plugin-first
- performance
---

## Why

Temperature, light, beauty, noise and danger are all scalar values over the grid. They should be one engine mechanism declared in data, not a special case each. Mods add layers and sources without engine changes.

## What

- `[[field]]` defs: outdoor ambient (set by scripts), indoor mode (room state, none, or same as outdoors), leak rate for room-state fields.
- `emit` on thing defs: amount and radius, fading with walking distance so walls block it. Stamped incrementally: adding or removing an emitter touches only its own cells; a wall change re-stamps only emitters in reach.
- Room-state fields hold a value per enclosed room that leaks toward outdoors and is pushed by emitters inside. O(rooms) per update. Survives room rebuilds by cell-weighted averaging.
- Fixed-point integer storage for determinism.
- Scripts: rim.field, rim.ambient, rim.set_ambient. Client: an overlay key cycles every field.

## Acceptance criteria

- [x] Emitter falloff follows walking distance and is blocked by walls
- [x] Adding, removing or walling off an emitter updates only nearby cells
- [x] Room values leak toward outdoors and rise with emitters inside
- [x] Room values survive a room rebuild elsewhere on the map
- [x] Temperature and light defined in core data; overlay shows any field
- [x] Tick cost measured before and after

## 2026-09-23

Built crates/rim_sim/src/field.rs. [[field]] defs (ambient, indoor = room|none|outdoor, leak_per_hour, room_gain, overlay range/colours, hud); emit = [{ field, amount, radius, cap }] on thing defs. Emitters stamped by walking-distance BFS (walls and doors block), each remembering its cells: add/remove costs its footprint; a wall change re-stamps only emitters in reach (test: far wall re-stamps 0 cells, near wall re-stamps only the one fire). Room fields: O(rooms) every 60 ticks, leak toward outdoors plus capped emitter heating, carried across room rebuilds by cell-weighted averaging. Bug found by tracing: room fields must not add the local stamp inside the room (double-counted fire, hut at 34°, over comfort). Core defines temperature and light; campfire emits both. Scripts rim.field/ambient/set_ambient; climate script drives day/night without sin/cos (lockstep). Client: O overlay cycles all fields, hover readings, outdoor temp in top bar. tests/fields.rs: 10 tests; autotest checks the overlay. Tick cost: mean 0.002 -> 0.004 ms, p99 0.005 -> 0.006 ms, max ~1.1 ms (5 days, seed 1). The added mean is mostly the comfort search; it uses a HashSet and could use a generation-stamped array like the pathfinder.
