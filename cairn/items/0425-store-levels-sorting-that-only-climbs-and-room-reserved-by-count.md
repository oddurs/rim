---
id: 425
uid: e953d711-8d06-43a5-b096-4ff7cce491f7
title: 'Store levels: sorting that only climbs, and room reserved by count'
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- 295
- 352
- 392
created: 2026-09-26
updated: 2026-09-28
closed_at: 2026-09-26
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
- [x] Haul search at or under 0.03 ms a step on the 8d551753 setup, measured and noted here
- [x] Determinism test passes

## 2026-09-26

Claimed ahead of its dependencies' merges: the ledger (ac643c1f) and the lost-overflow fix (629e1fa7) are in review, and this branch stacks on the ledger.

## 2026-09-26

Measured, cargo run --release -p rim_sim --example bench -- --haul --days 0.25 (the 8d551753 setup: 250x250, 30 colonists, 200 pawns, a 40x40 stockpile, 300 loose stacks), main then this branch, twice each, back to back: mean tick 0.170/0.114 ms -> 0.047/0.041 ms; pawns 0.166/0.111 -> 0.043/0.038 ms; p99 4.8/3.2 ms -> 0.27/0.26 ms. The whole pawns system is now around 0.04 ms, so the haul search is within the 0.03 ms target's reach but isn't measured on its own; the tick and pawns numbers are what's measured. Load ~40-70 from other builds during the runs.

## 2026-09-28

Measured 2026-09-28 on main c30d53a3 with find_haul timed in isolation (throwaway instrumentation, not committed): bench --haul --days 0.25 (250x250, 30 colonists, 200 pawns, 40x40 stockpile, 300 loose stacks), 5000 ticks, 187 searches. Four runs at load 37-42: 0.0124, 0.0028, 0.0063, 0.0059 ms a tick. Under the 0.03 target on every run. The whole pawns system read 0.11-0.16 ms a tick in the same runs; that is pathing, not the haul search.
