---
id: c1089360-a315-4641-9b99-d4d4d33cecda
title: 'Pathfinding on a map full of work: long A* searches in forest and rock'
type: perf
status: backlog
milestone: scale
created: 2026-09-25
updated: 2026-09-25
priority: p1
api: none
effort: m
layer: engine
area: pathing
pillar:
- performance
---

## Budget

The stress bench (every cell designated) within the 2 ms tick budget
(DESIGN.md §8); today it's 6-8 ms mean with p99 over 25 ms.

## Measurement (before)

`bench --designate-all --days 0.5` (250x250, 30 colonists, 200 pawns,
release, reference machine): mean tick 6.1 ms. A macOS `sample` puts 98% of
it under `Pathfinder::find`. Searches expand 18,257 nodes on average (cap
30,000), against 23 on an ordinary colony. Choosing work is 0.047 ms of it
(measured for 0e73145a).

## Approach

Find out which searches are long: failed ones at the cap (reachable by
region but not by path, e.g. Touch goals inside a forest), or long real
paths the octile heuristic explores badly through obstacles. Then the cheap
fix first (a tighter cap or reachability check for the failing ones), and
hierarchical pathing (DESIGN.md §8, "later, behind the same interface")
only if the long real paths are what's left.

## Acceptance criteria

- [ ] The long searches are explained, with counts, in a note here
- [ ] The stress bench's mean tick is within budget, before and after recorded
