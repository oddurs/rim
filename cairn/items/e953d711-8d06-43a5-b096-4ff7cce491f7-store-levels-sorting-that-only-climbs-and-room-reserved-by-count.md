---
id: e953d711-8d06-43a5-b096-4ff7cce491f7
title: 'Store levels: sorting that only climbs, and room reserved by count'
type: feature
status: review
milestone: crafting
assignee: Oddur Sigurdsson
claimed: 2026-09-26
depends_on:
- 01691032-1f99-475a-b964-6e3f4149fe22
- 629e1fa7-7e3c-4c42-96dd-98ea8a3b762f
- ac643c1f-3f5b-4908-884f-da4e87bfcf01
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: additive
effort: l
layer: engine
area: ai
---

## Why

A stack in a zone that allows it is never moved again, so nothing ever re-sorts (8d551753). The haul search walks every thing and every zone cell: 0.33 ms a step at 250×250 with 30 colonists and 300 loose stacks, over the 0.2 ms work-choice budget (§4d). Destinations aren't reserved, so `find_haul` scans other pawns' jobs.

## What

- `[[store_priority]]` in core: five levels, Normal in the middle; zones get a level (default Normal) and a `ZoneLevel` command.
- Sorting rules from DESIGN.md §4f: the highest level that takes a stack and has room wins, nearest within a level; a stored stack moves only to a strictly higher level; a store that stops taking a thing lets it go.
- Indexes: `accepts` by thing and level, `room`, `unsorted` by chunk, `waiting` by thing. All derived, ordered by id.
- Room reserved by count on the store, replacing the scan of other pawns' haul jobs.
- The why panel says why a stack stays, moves or waits.

## Acceptance criteria

- [x] A stack in a Normal zone moves to a Preferred one that takes it; never between two Normal zones (tests)
- [x] Changing a zone's filter sends what it no longer takes to the best place that does (test)
- [x] Two haulers never fill the same last cell (test)
- [ ] Haul search at or under 0.03 ms a step on the 8d551753 setup, measured and noted here
- [x] Determinism test passes

## 2026-09-26

Claimed ahead of its dependencies' merges: the ledger (ac643c1f) and the lost-overflow fix (629e1fa7) are in review, and this branch stacks on the ledger.

## 2026-09-26

Measured, cargo run --release -p rim_sim --example bench -- --haul --days 0.25 (the 8d551753 setup: 250x250, 30 colonists, 200 pawns, a 40x40 stockpile, 300 loose stacks), main then this branch, twice each, back to back: mean tick 0.170/0.114 ms -> 0.047/0.041 ms; pawns 0.166/0.111 -> 0.043/0.038 ms; p99 4.8/3.2 ms -> 0.27/0.26 ms. The whole pawns system is now around 0.04 ms, so the haul search is within the 0.03 ms target's reach but isn't measured on its own; the tick and pawns numbers are what's measured. Load ~40-70 from other builds during the runs.
