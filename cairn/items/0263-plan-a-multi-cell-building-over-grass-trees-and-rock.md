---
id: 263
uid: 26ba97aa-2069-4617-9af0-9cd3df8c8b16
title: Plan a multi-cell building over grass, trees and rock
type: feature
status: done
milestone: houses
assignee: Oddur Sigurdsson
depends_on:
- 239
created: 2026-09-25
updated: 2026-09-26
closed_at: 2026-09-26
priority: p2
api: none
effort: m
layer: engine
area: building
---

## Why

Planning a building over a natural thing (#116) marks it to be cleared and puts the blueprint up once it's gone, but only for the one cell being planned. A multi-cell thing (8cf4db07) needs its whole footprint clear, so today a 2x1 planned where the second cell has grass is refused. In the stone age most ground has grass, and no shipped thing is bigger than one cell yet.

## What

- The Build command plans a multi-cell thing when every footprint cell is open or holds something natural it can clear.
- Each natural thing in the footprint is marked to be cleared; the blueprint goes up when the last one is gone.
- Cancelling clears every mark.

## Acceptance criteria

- [x] A 2x1 planned over grass and a tree goes up once both are cleared (test)
- [x] Cancelling leaves the grass and the tree unmarked

## 2026-09-26

Moved from building, which had already shipped when this was filed: multi-cell workbenches arrive with crafting.

## 2026-09-26

Moved to houses: facing (ba18a8e4) makes 1x2 beds and 2x1 benches the norm, and they must be plannable over grass.

## 2026-09-26

Planned carries the building's anchor when it covers more than its own cell; the blueprint's spawn is tried whenever one is cleared, and spawn_fixture already refuses while any footprint cell is occupied, so it goes up with the last. Cancelling any marked cell unplans its siblings (World::unplan).
