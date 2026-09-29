---
id: 3cb3dd5f-3517-4870-9d89-cda68899f9eb
title: The path heuristic overestimates where floors cost under 100
type: bug
status: backlog
milestone: bare-metal
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
