---
id: 4610c342-d79d-48a1-b843-1f064dbab668
title: Any wall change rebuilds every room on every level, and every room's boundary
type: perf
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
layer: engine
area: sim
---

## What

A wall, door, support, water footing or terrain change sets `rooms_dirty`. The next `ensure_rooms` (map.rs ~880) then does all of this from scratch:
- clears and re-floods the room grid over every cell of every level (`for start in 0..self.room.len()`), and spreads cover;
- `refresh_boundaries` (world.rs ~2972) looks up every boundary piece of every room, the outdoors included (every wall and rock face on the map), once per field;
- `Fields::carry_rooms_over` (field.rs ~808) walks every cell of the map once per room field.

## Why it matters

The cost is O(map cells × levels + boundary cells × fields) per change, not per changed room. A colony building walls pays it every few ticks, and it grows with map size and depth, not with what changed. It runs on the tick path (`Sim::step` "rooms", "boundary", "fields").

## Direction

Rebuild only the rooms on the changed cell's level, flooding from the changed cells' neighbours (a split or merge touches the rooms beside the change). Keep ids stable for untouched rooms, so `carry_rooms_over` and the boundary sums redo only the rooms that changed. The save/load twin must still hold: a load builds from scratch and must match.

## Acceptance

- [ ] A wall placed or removed costs work proportional to the rooms it touches, not the map
- [ ] Tests show the incremental result equals a from-scratch rebuild, over random edits (the pattern of `patched_cover_matches_the_whole_map`)
- [ ] The scaling bench (981e6792) shows rooms growing linearly or better with map size
