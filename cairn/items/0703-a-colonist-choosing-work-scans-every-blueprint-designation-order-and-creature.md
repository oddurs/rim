---
id: 703
uid: fdd1fd2a-ba01-4451-a380-c88b0e2a8885
title: A colonist choosing work scans every blueprint, designation, order and creature
type: perf
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
layer: engine
area: ai
---

## What

`ai::choose_work` (ai.rs ~841) runs for every colonist that thinks. Each time it iterates:
- every `Blueprint`, every `Designated` thing and every `Order` in the world (three ECS queries);
- every pawn (designated creatures);
- then `find_haul`, with `Bound::of` (ai.rs ~1185) walking every pawn to see what haulers are bound for.

Items are found through the stock's chunk holdings, nearest first, but the job sources aren't: a colony with a thousand trees marked for chopping scans a thousand designations per colonist per think.

## Why it matters

The cost is O(colonists × jobs) plus O(colonists × pawns) per round of thinking, which grows as the square of colony size. `who_takes` and the why panel call the same function.

## Direction

Index job sources by work type and chunk (designations, blueprints and orders kept as they're added and removed, as `StoreIndex` does for stacks), and search nearest chunks first with the existing early stop. Keep "bound for" per container or cell as haul jobs start and end, instead of scanning pawns.

## Acceptance

- [ ] Choosing work costs time in the jobs near the colonist, not in all jobs
- [ ] The same choice as today on the seed corpus (a test compares with a brute-force choice)
- [ ] The scaling bench shows work choice growing linearly with colonists
