---
id: 3ccab46f-32d5-4248-a23d-88cdc8b3ec70
title: Carried things keep what they're made of
type: bug
status: backlog
milestone: stone-age
created: 2026-09-25
updated: 2026-09-25
priority: p1
api: none
effort: m
layer: engine
area: sim
---

## Why

A pawn carries `(def, count)` (`Pawn.carry`). Everything carried loses what it's made of and its hp: a flint hand axe hauled to a stockpile (#102) or supplied to a work order (74b6fa7e) is set down as a plain one at full hp, and a stack of flint-made things merges into one of bone. With tools that wear and take their quality from material (7016d86b), that's the stone age's economy leaking.

## What

- Carry what a stack is made of, and its hp when it's one thing (a tool): `carry` gains the material, or a carried single thing stays the same entity.
- `put_item` and `place_item_of` merge only with the same material, and a set-down thing keeps its hp.
- A work order's `delivered` records the material, so `order_done.stuff` sees it.

## Acceptance criteria

- [ ] A hauled flint axe is still a flint axe, with the hp it had
- [ ] Stacks of one thing in two materials never merge by hauling
- [ ] A tool supplied to a work order reports its material in `order_done`
