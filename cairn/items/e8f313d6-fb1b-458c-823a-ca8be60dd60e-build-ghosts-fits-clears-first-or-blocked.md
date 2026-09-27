---
id: e8f313d6-fb1b-458c-823a-ca8be60dd60e
title: 'Build ghosts: fits, clears first, or blocked'
type: feature
status: planned
milestone: chalkline
depends_on:
- cd59b515-b98c-4f24-92f8-ec55161d22aa
- d83192ed-0a3e-4a6f-a39a-04ce94a68935
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: none
effort: m
layer: client
area: building
---

## Why

A wall drag fills the perimeter at 35% in the tool's colour. The cursor is a 2 px square outline. Neither says which cells will actually get a wall, which cells need a tree cleared first, or why a cell is refused. DESIGN.md §6f.

## What

- Hover and drag both draw `build_preview`, one ghost per cell:
  - `Open`: an `intent_fill` wash and a 1.25 px `accent` edge.
  - `Clears`: the same, plus a `bad` triangle at the top-left.
  - `Blocked`: a `threat` wash at 13% and a cross.
- A thing bigger than a cell is drawn as one ghost over its footprint, turned by `build_facing`, with a chevron at its foot showing which way it faces. `T` turns it.
- Chip:
  - During a drag: "18 walls · 36 logs", or "16 of 18 walls" when some cells are blocked. The cost comes from the def and the chosen material.
  - Hovering a blocked cell: "Blocked by <label>" in threat.
- A click where every cell is blocked shakes the ghost twice by 3 px over 180 ms and keeps the chip for 1.4 s.
- This replaces the perimeter fill and the tool-hover square in `world_ui`.

## Acceptance criteria

- [ ] Autotest: a wall drag across one water cell and one tree gives one blocked ghost, one clears ghost and the rest open, with a chip reading "N of M walls"
- [ ] Autotest: a bed ghost over a table is blocked, and the chip names the table
- [ ] Autotest: turning with `T` swaps the bed ghost's footprint between 1×2 and 2×1
- [ ] After release, the cells planned match the open and clears ghosts (autotest)
- [ ] Screenshots `chalk-build-run` and `chalk-build-blocked`
