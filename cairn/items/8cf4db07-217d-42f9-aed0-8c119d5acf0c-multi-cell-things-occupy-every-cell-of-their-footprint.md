---
id: 8cf4db07-217d-42f9-aed0-8c119d5acf0c
title: Multi-cell things occupy every cell of their footprint
type: feature
status: backlog
milestone: colony
created: 2026-09-24
updated: 2026-09-24
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

- [ ] A 2×1 thing blocks both cells and is one entity
- [ ] Removing it frees both cells and regions update
- [ ] Determinism test passes
