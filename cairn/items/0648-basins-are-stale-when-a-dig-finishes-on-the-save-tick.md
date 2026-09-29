---
id: 648
uid: 8d246327-85c1-45bb-b1d5-03249e56c56b
title: Basins are stale when a dig finishes on the save tick
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
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

- [ ] A save on the tick a dig finishes loads into the same water and news
- [ ] A test that fails before the fix and passes after
