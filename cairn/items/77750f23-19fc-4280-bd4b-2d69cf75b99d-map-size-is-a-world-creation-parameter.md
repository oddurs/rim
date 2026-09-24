---
id: 77750f23-19fc-4280-bd4b-2d69cf75b99d
title: Map size is a world-creation parameter
type: feature
status: backlog
milestone: world
created: 2026-09-24
updated: 2026-09-24
priority: p2
api: additive
effort: s
layer: engine
area: map
pillar:
- plugin-first
---

## Why

`MAP_SIZE` is a constant in `sim.rs`. DESIGN.md §6a says the map's
dimensions come from world creation, so a biome or plugin can ask for a
different size, and the benchmark (0093) can ask for 250×250 without a
code change. The edge stays a hard boundary whatever the size.

## What

- Width and height on the world-creation command, with core's default in
  data.
- Every buffer sized from the map (regions, rooms, fields, pathfinder
  scratch, client caches) takes it from the map, not a constant.
- Autotest and the balance harness accept a size.

## Acceptance criteria

- [ ] `MAP_SIZE` is gone
- [ ] Determinism test passes at two sizes
- [ ] A non-square map plays and renders correctly
