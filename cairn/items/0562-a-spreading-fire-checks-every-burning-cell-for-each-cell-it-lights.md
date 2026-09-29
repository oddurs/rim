---
id: 562
uid: 098fd513-3e0a-4cfe-9ed6-9cee0fb27f32
title: A spreading fire checks every burning cell for each cell it lights
type: perf
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
layer: plugin
area: perf
---

## What

`light` in mods/fire/scripts/fire.luau walks the whole burning list to refuse a cell that's already alight. `fire.step` calls it for every cell that caught this step (`for _, c in caught do light(still, ...)`), even though the step already keeps a `lit` set of every burning cell.

## Why it matters

The cost is O(caught × burning) per step. A wildfire of a few thousand cells spreading a few hundred a step costs about a million comparisons every 250 ticks, all in Luau, inside the step's hook budget.

## Direction

Pass the step's `lit` set to `light`, or keep a set beside the list, so the check is one lookup. `fire.ignite` outside a step can build the set once.

## Acceptance

- [ ] Lighting a cell costs a lookup, not a scan of the fire
- [ ] Fire's existing tests pass unchanged (the same cells burn in the same order)
