---
id: 8cf4db07-217d-42f9-aed0-8c119d5acf0c
title: Multi-cell things occupy every cell of their footprint
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
created: 2026-09-24
updated: 2026-09-25
closed_at: 2026-09-25
priority: p3
api: additive
effort: m
layer: engine
area: building
pillar:
- plugin-first
---

## Why

A long table, a 3×3 bench or a double bed needs more than one cell.
DESIGN.md §6a rules that a multi-cell thing occupies each of its cells in
the fixture layer, all pointing at one entity, so pathing, rooms and
fields never learn about footprints.

## What

- `size = [w, h]` on a thing def; default 1×1.
- Placement checks every cell; build, deconstruct and destroy clear them all.
- Interaction spots (0218) are offsets from the anchor cell.
- The renderer draws the look once, over the whole footprint.

## Acceptance criteria

- [x] A 2×1 thing blocks both cells and is one entity
- [x] Removing it frees both cells and regions update
- [x] Determinism test passes

## 2026-09-25

Built in the engine only. ThingDef.size = [w, h] (default [1, 1], each 1 to 8; items and floors stay one cell), anchored at Thing.pos and extending right and down; ThingDef::footprint lists its cells. Every place that sets or clears the fixture layer covers the footprint: spawn_fixture_of (every extra cell must be in bounds, passable and free, or nothing is placed), complete_building, despawn_thing (only cells still holding the entity) and snapshot restore. Pathing, rooms, boundaries and shelter read the fixture layer cell by cell, so they need nothing. Interaction spots were already offsets from the anchor. The Build command already steps cells and a spawn refuses taken ones, so a drag of wide things places them side by side. Drawing is split to f607a83d (the renderer draws per cell; no shipped thing is bigger than one cell yet), in rim-c2's area.

## 2026-09-25

Review fixes: path::Goal gains Area { at, size }: stand on or next to any cell of a footprint (A* heads for the nearest cell of it, can_reach checks around every cell), and World::reach_goal gives Touch for a one-cell thing and Area for a bigger one; every work goal on a fixture (construct, deliver, supply, craft, harvest, deconstruct, breach, orders, work_blocked) uses it, so a 2x1 walled in at its anchor is worked from its far end. A pawn anywhere in a plan's footprint steps out before it's finished, and anyone caught inside a finished thing is moved to the nearest open cell ring by ring (from the middle of a 3x3 the neighbours are the thing). Planning a multi-cell thing over grass or trees (#116 clears one cell) is split to 26ba97aa.
