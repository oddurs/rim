---
id: 8d246327-85c1-45bb-b1d5-03249e56c56b
title: Basins are stale when a dig finishes on the save tick
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-29
closed_at: 2026-09-28
priority: p2
api: none
layer: engine
area: save
---

## What

`update_water` runs at the start of a step, while `complete_building` → `dig` runs later, in the AI phase. So a snapshot taken after a step in which a dig finished holds basins from before the dig.
- The live game's next rebuild (water.rs ~201) splits the volume by cell share, carries wet cells, and emits `Breach` for a newly fed basin.
- The load path (water.rs ~477-491) rebuilds quietly, and puts each saved volume into whichever new basin holds the old basin's lowest cell: no split, the excess clamped away, and no `Breach`.

## How it fails

The live and loaded games diverge in water and in breach news.

## Reproduce

Not yet run; verified by reading. tests/basins.rs `an_aquifer_breach_at_minus_two_fills_minus_three_first`: save on the tick the dig completes, then compare events and per-cell water depth after one step.

## Fix

Save each basin's cells (or its build revision), so the load rebuilds from the same basins the live game had and then updates as the live game will. Or rebuild water at the end of the step too.

## Acceptance

- [x] A save on the tick a dig finishes loads into the same water and news
- [x] A test that fails before the fix and passes after

## 2026-09-28

Fixed by rebuilding basins at the end of each step as well as the start (World::rebuild_water), so a save between steps holds the basins the next step would find. A Breach from that rebuild is pushed after the step's dispatch and dispatched next step, as before. The start-of-step rebuild stays for anything that changes the ground before it (commands). Cost: one revision compare per level per tick. The test compares the water and world sections, not the whole snapshot: engine:fields differs after a load for another reason (the ground-on-load item, c0042c1f).

## 2026-09-29

PAUSED at the merge freeze (main d633f7b5): fix and test done (test fails before, passes after), committed on fix/8d246327-water-at-step-end and rebased on origin/main before the freeze; the full gate has not run on this commit. Next: rebase onto main, run scripts/task check, mark the PR ready.
