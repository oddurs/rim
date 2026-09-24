---
id: 8dcec2bf-1eea-48ff-8d66-a36de615a9d6
title: Data-driven map generation
type: feature
status: done
milestone: foundations
created: 2026-09-22
updated: 2026-09-22
priority: p1
api: additive
effort: m
layer: engine
area: map
pillar:
- plugin-first
---

## Why

Terrain bands, plant density and wildlife come from defs so mods can reshape the world without code.

## Acceptance criteria

- [x] [terrain.gen] elevation and moisture bands with priority
- [x] [thing.spawn] densities
- [x] [creature.spawn] groups
- [x] Start cell lies in the largest connected region
