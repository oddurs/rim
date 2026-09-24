---
id: f51af7ba-8fba-4127-bda0-07387348a786
title: 'Room detection: enclosed regions bounded by walls and doors'
type: feature
status: done
milestone: shelter
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p0
api: additive
effort: m
layer: engine
area: map
pillar:
- survival
- performance
---

## Why

Rooms are regions that do not touch the map edge. Recomputed only when walls change.

## Acceptance criteria

- [x] Rooms exposed to scripts
- [x] Doors split regions for rooms but not for pathing

## 2026-09-23

Map::ensure_rooms: 4-connected flood fill over passable non-door cells, rebuilt only when a fixture's blocks/door status or terrain changes (tested: items, pawns and 500 ticks cause no rebuild; one wall causes exactly one). Room { id, cells, touches_edge }; enclosed() = !touches_edge && cells <= MAX_ROOM_CELLS (400). New def flag thing.door; core's wooden door sets it. Scripts: rim.indoors(x, y), rim.room_at(x, y) -> { id, cells, enclosed } or nil. Client hover shows indoors/outdoors. tests/rooms.rs: 6 tests incl. a probe mod calling the script API; mutation-checked (doors not bounding rooms fails 2 tests, no size cap fails 1).
