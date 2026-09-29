---
id: 185
uid: 9b569a33-c488-42df-84c2-9bfa83a14b3e
title: Terrain properties, tags and distance-to inputs
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- 181
created: 2026-09-23
updated: 2026-09-27
closed_at: 2026-09-27
priority: p0
api: additive
effort: s
layer: engine
area: map
pillar:
- plugin-first
- performance
---

## Why

Ground state and plant growth vary across the map because the ground does: sand drains, marsh holds water, soil near a river stays damp. Terrain needs named properties the term language can read, and "how far is the nearest water" must be O(1).

## What

- `[[terrain]]` gains `props = { fertility, drainage, water_table, ... }` (any names; missing ones read 0 with a load warning if referenced) and `tags = ["water"]`.
- `terrain = prop` and `near = tag` term inputs. `near` reads a distance grid built by multi-source BFS at load, capped at 16 cells, and patched locally when terrain changes.
- Core values for every terrain (fertility: rich soil 1.4, grass 1.0, dirt 0.7, sand 0.2, marsh 0.8; drainage and water table to match).
- Scripts: `rim.terrain_prop(x, y, name)`. Hover readout shows fertility.
- Replaces 0110's "fertility from terrain" criterion.

## Acceptance criteria

- [x] Terrain props and tags load from data; a mod can add a prop and read it in terms
- [x] `near` distances are correct after a terrain change (test) and cost only the changed area
- [x] Core terrain has fertility, drainage and water table

## 2026-09-23

Moved to Crafting with the lean Weather sprint: nothing reads terrain properties until farming (0110).

## 2026-09-27

Props are bare names shared by all mods, like tags, so any mod can read core's fertility. They compile to indices into DefDb::terrain_props, and each terrain keeps its values in fixed point. A read is one lookup, and a prop the terrain doesn't give reads 0. A prop or tag no terrain has is a load warning, not an error, because a mod may read one that another optional mod adds. near uses Chebyshev distance on the cell's own level and ignores walls: it measures how far the ground is, not the walk. Grids exist only for tags a term reads, and core reads none yet, so none are kept until wetness lands. A change patches the cells within 16 by searching the box within 32. That is at most 65×65 = 4225 cells per changed cell and tag; tests/terrain_props.rs asserts the bound and compares every distance with brute force. Past a plane's worth of changes, one full pass is cheaper and runs instead. The hover readout is data: core's derived field fertility (terrain fertility ×100, in %) shows on the hover card and the O overlay, with no engine code naming it.
