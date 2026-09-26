---
id: 2d44949e-f733-470e-ac34-375eac79ab43
title: 'Work Board: drag columns to set the colony''s tie-break'
type: feature
status: backlog
milestone: mood
depends_on:
- f1b96df4-ff22-4c7c-88b4-15bd0c6391a2
created: 2026-09-25
updated: 2026-09-25
priority: p3
api: additive
effort: s
layer: engine
area: ui
---

## Why

`order` breaks ties between work types at the same level, and DESIGN.md §4d lets the player drag it. It is a def value today, so the board (f1b96df4) shows the order but can't change it.

## What

- A colony-level override of the work-type order, in the world, the state hash and the save, changed by a command.
- The board's column headers drag to reorder, and find_work uses the colony's order.

## Acceptance criteria

- [ ] Dragging a column changes which of two equal-level jobs a colonist takes (test)
- [ ] The order is saved and hashes
