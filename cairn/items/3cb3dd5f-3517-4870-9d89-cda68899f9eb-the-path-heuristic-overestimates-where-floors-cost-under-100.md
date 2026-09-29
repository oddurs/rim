---
id: 3cb3dd5f-3517-4870-9d89-cda68899f9eb
title: The path heuristic overestimates where floors cost under 100
type: bug
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
claimed: 2026-09-28
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: engine
area: pathing
---

## What

A* uses `octile` (10 straight, 14 diagonal per cell) as its heuristic. `Map::cost` lets a floor's cost replace the ground's, and core has floors under 100: a bridge at 90, and buildings.toml floors at 70 and 80. Over those, a step costs less than the heuristic assumes, so the heuristic isn't admissible and A* can return a longer path than the best one. Deterministic, so not a desync; just worse paths over floors.

This is path.rs, where quiet-field's landmark heuristic work is in flight (a708c037). Whoever owns it should keep the heuristic admissible: scale by the cheapest step cost the defs allow.

## Reproduce

Verified by reading. A corridor of cost-70 floor beside open ground: A* may take the ground.

## Acceptance

- [ ] The heuristic never overestimates, given the loaded defs
- [ ] A test comparing A* with Dijkstra over mixed floor costs

## 2026-09-28

Measured 2026-09-28 on a708c037's branch (octile plus landmarks), with a knob scaling the heuristic (0% is Dijkstra, the exact reference). A lake seed, 300 pairs within 50 cells of the colony, core's floor at 70. (1) No floors: today's heuristic returns the optimal path on all 300. Scaling it to be admissible over floors costs 4.51x the nodes at 0.70 and 5.07x at 0.64. (2) 45% of cells floored, scattered: today 296 of 300 paths are longer than the best, mean 1.080x, worst 1.212x; admissible costs 6.05x the nodes (0.70) and 8.95x (0.64). (3) The area fully floored: today 163 of 300 longer, mean 1.013x, worst 1.085x; admissible costs 2.13x (0.70) and 9.89x (0.64). 0.70 is NOT enough: a diagonal step onto a 70 floor costs 14*70/100 = 9 after integer truncation, so the cheapest step is 9/14 of the heuristic's, and 0.70 still left 131 and 4 paths off-optimal in (2) and (3). The exact scale is 0.64 (the cheapest (step*cost/100)/step over the defs' floors). Conclusion: exact paths over floors cost 5-10x search work everywhere, including maps with no floors at all, to save about 1-8% of walking over floored areas (21% at worst in a pathological half-floored pattern). Under Bare Metal the heuristic should stay as it is; the item is a decision, not a bug.

## Proposed status: doing -> dropped (Oddur Sigurdsson, 2026-09-28)

Measured: an admissible heuristic over floors under 100 costs 5-10x the search nodes on every map, floors or none, to shorten walks over floored ground by about 1-8% (worst 21% in a half-floored checkerboard). Bare Metal puts speed first; keeping today's heuristic, and documenting the bounded detour, is the trade.
