---
id: acd85584-7f3d-4348-8d56-5242a0bdb620
title: 'Portals: stairs, ladders, digging down, and paths across levels'
type: feature
status: backlog
milestone: depth
depends_on:
- 3f90e043-bf62-48c8-ac67-d043dc755b6e
- c1089360-a315-4641-9b99-d4d4d33cecda
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: l
layer: engine
area: pathing
pillar:
- performance
---

## Why

Levels meet only at portals (DESIGN.md §6d). This item is how a colonist gets down, and how the engine knows it can: reachability must stay O(1) and a path must stay one plane at a time.

## What

- A thing def with `portal = { dir, cost }` covers two cells, one above the other, like a multi-cell footprint. Both ends are one entity. An owned portal is locked to other factions, like a door (`locked_against`).
- Core: `stairs` and `ladder` (cheaper to build, slower to climb).
- Designations: **dig down** turns an open cell's floor to air and mines the cell below; **dig stairs** does the same and leaves a stair pair. Both go exactly one level, from a neighbouring cell, with the lower stratum's `requires` tags.
- Reachability: a union-find over `(z, region)` joined at portals, rebuilt in O(portals) after a level's regions rebuild or a portal changes. `can_reach` compares two ids.
- Pathing leg by leg: Dijkstra over the portal graph (edges are octile distance between portals in one region), then A* on one plane to the next portal with the existing scratch buffers. Each leg is planned on arrival.
- Job search weighs work on another level by its portal-graph distance.
- Depends on the long-search fix (`c1089360`): pathfinding is already where the stress bench spends its time.

## Acceptance criteria

- [ ] A colonist on the surface fetches stone from −3 through two stairwells (scene test)
- [ ] `can_reach` across levels is O(1); rebuilding reach costs under 0.05 ms with 60 portals, recorded here
- [ ] Bench with a three-level colony: mean and p99 per tick and nodes expanded per search, recorded here and in §6d
- [ ] A raider can't use a colony's owned stairs
- [ ] Core alone digs down to −3 (moved here from the strata item, where nothing could dig yet)
- [ ] Determinism test passes
