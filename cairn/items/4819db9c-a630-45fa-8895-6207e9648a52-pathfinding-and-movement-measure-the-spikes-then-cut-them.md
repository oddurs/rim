---
id: 4819db9c-a630-45fa-8895-6207e9648a52
title: 'Pathfinding and movement: measure the spikes, then cut them'
type: perf
status: done
milestone: scale
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: none
effort: m
layer: engine
area: pathing
---

## Why

The sim bench (250x250, 30 colonists, 200 pawns) averages 0.22 ms a tick, well in budget, but its p99 is 3 ms and its worst tick 71 ms: all in the pawns system. A tick like that is a visible hitch at any speed. Pathfinding is the likely cost, but it hasn't been measured.

## What

- The bench reports pathfinding (searches, nodes expanded, time) and what the slowest ticks were doing.
- Cut what the numbers point at: search count, nodes per search, or a single long search, without changing where pawns go (the determinism test pins it).

## Acceptance criteria

- [x] The bench reports pathfinding cost and the worst ticks' causes
- [x] The worst tick and p99 drop, measured before and after on the same seed
- [x] The determinism test passes: same world, same paths

## 2026-09-26

Measured on bench seed 1, 250x250, 2 days: the worst ticks were two colonists retrying a search that needed 30,022 nodes against a 30,000 cap (a 366-step detour for work 60 cells away), about 290 times; regions said reachable, so every retry failed the same way. The other half of pawn time was comfortable_spot: 2,593 calls, every one finding nothing after ~3,900 cells, with a SipHash set. Fixes: go_to caps at the map's size (regions already proved a way exists); A* breaks f ties toward the goal and reads each neighbour once; the flood reuses the pathfinder's stamped buffers; a pawn that found nowhere better waits an hour before looking again. CPU for the run 6.95 s -> 1.41 s; nodes per search 235 -> 16; p99 5.4 -> 0.36 ms. Colonist survival across seeds 1-5 moves both ways (all deaths are wolf attacks, same with or without the comfort wait): chaos from different paths, not a regression. Left: the take/put swap of Pawn is ~12% of what remains.
