---
id: 26ba97aa-2069-4617-9af0-9cd3df8c8b16
title: Plan a multi-cell building over grass, trees and rock
type: feature
status: backlog
milestone: crafting
depends_on:
- 8cf4db07-217d-42f9-aed0-8c119d5acf0c
created: 2026-09-25
updated: 2026-09-26
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

- [ ] A 2x1 planned over grass and a tree goes up once both are cleared (test)
- [ ] Cancelling leaves the grass and the tree unmarked

## 2026-09-26

Moved from building, which had already shipped when this was filed: multi-cell workbenches arrive with crafting.
