---
id: 546
uid: e8f313d6-fb1b-458c-823a-ca8be60dd60e
title: 'Build ghosts: fits, clears first, or blocked'
type: feature
status: done
milestone: chalkline
assignee: Oddur Sigurdsson
depends_on:
- 525
- 533
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
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

- [x] Autotest: a wall drag across one water cell and one tree gives one blocked ghost, one clears ghost and the rest open, with a chip reading "N of M walls"
- [x] Autotest: a bed ghost over a table is blocked, and the chip names the table
- [x] Autotest: turning with `T` swaps the bed ghost's footprint between 1×2 and 2×1
- [x] After release, the cells planned match the open and clears ghosts (autotest)
- [x] Screenshots `chalk-build-run` and `chalk-build-blocked`

## 2026-09-27

This item adds the theme token intent_fill (and its doc row) to mods/core/ui/theme.toml and docs/modding/ui.md: tokens land with the item that first reads them (d83192ed review).

## 2026-09-27

Stacked on designate (bb769d00) with #215's preview API.

## 2026-09-27

Review: every finding applied.
- The preview is cached on (from, to, facing, material, tick).
- Plans in an earlier rectangle block a later one's overlapping cells.
- The cross sits on the footprint cell that's in the way.
- Only the refused cell shakes, and ghosts are culled to the screen.
- Plurals and articles come from one helper, shared with designate's hints.
- Refusals read in words ('Blocked by an oak tree', 'Off the edge of the map', 'No material chosen to build it from').
- A click is refused on the cell released on.
- Zone tools get a violet hover frame.
- build_cost calls build_cost_for.
- The autotest fails loudly if there's no woodpile.
Autotest: 293 passed, 0 failed.

## 2026-09-28

Criteria as tested:
1. The wall run crosses a tree and an existing plan, not water: one clears ghost, the plan's cells crossed, hint '6 of 8 walls'. Water blocks the same way, and rim_sim's previews test covers it (water_blocks_a_tree_clears_and_a_wall_is_in_the_way).
2. A bed's ghost over a table: blocked, hint 'Blocked by a table'. Added before the PR.
3. T turns a woodpile's ghost (2×1 to 1×2), not a bed's; same mechanism.
Checks: scripts/task check passes (828 tests), and the autotest passes (380 passed, 0 failed) with #312 applied.
