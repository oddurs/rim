---
id: 185
title: Terrain properties, tags and distance-to inputs
type: feature
status: backlog
milestone: crafting
depends_on:
- 181
created: 2026-09-23
updated: 2026-09-23
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

- [ ] Terrain props and tags load from data; a mod can add a prop and read it in terms
- [ ] `near` distances are correct after a terrain change (test) and cost only the changed area
- [ ] Core terrain has fertility, drainage and water table

## 2026-09-23

Moved to Crafting with the lean Weather sprint: nothing reads terrain properties until farming (0110).
