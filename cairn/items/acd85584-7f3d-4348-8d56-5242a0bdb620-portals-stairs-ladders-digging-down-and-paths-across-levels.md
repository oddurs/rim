---
id: acd85584-7f3d-4348-8d56-5242a0bdb620
title: 'Portals: stairs, ladders, digging down, and paths across levels'
type: feature
status: review
milestone: depth
assignee: Oddur Sigurdsson
claimed: 2026-09-27
depends_on:
- 3f90e043-bf62-48c8-ac67-d043dc755b6e
- c1089360-a315-4641-9b99-d4d4d33cecda
created: 2026-09-26
updated: 2026-09-27
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

- [x] A colonist on the surface fetches stone from −3 through two stairwells (scene test)
- [x] `can_reach` across levels is O(1); rebuilding reach costs under 0.05 ms with 60 portals, recorded here
- [x] Bench with a three-level colony: mean and p99 per tick and nodes expanded per search, recorded here and in §6d
- [x] A raider can't use a colony's owned stairs
- [x] Core alone digs down to −3 (moved here from the strata item, where nothing could dig yet)
- [x] Determinism test passes

## 2026-09-27

Built as builds rather than designations: stairs, ladder and pit are [[thing]]s whose build has dig = {} (stairs, ladder: the thing stays and is a portal) or dig = { hole = "air" } (pit: the cell becomes air). A dig's work and tool are the rock below's destroying harvest, added to the build's (World::build_requires, stat 'work'), so primitive's gates reach digging with no new patch. Portals: Map::portals() (sorted by top cell) and a per-cell link; reach is regions joined at portals with a union-find per faction (owned portals are walls to others); A* takes one extra edge through a portal at its cost. Map::air_cells(z) and a per-level revision serve lighting (3124bd7b). Measured: joining reach over 61 portals on 192^2 x 4 levels, 0.0017 ms (best of 200). bench --levels (two dug rooms below the colony, every colonist mining underground), 0.5 day: mean 0.41-0.52 ms, p99 5.6-11 ms, 12 nodes per search; rooms ~0.2 ms is the largest cost (see 66906291). Surface only, 3 interleaved pairs with main: mean 0.162/0.195/0.150 vs 0.178/0.231/0.189 ms, p99 1.07/1.17/0.80 vs 1.29/4.08/1.40. Two fixes on the way: wind shelter walked every level and wrote underground rock's lee onto the surface (it's now the surface's only, keyed on its own revision), and roof cover was worked out for rock underground (it's now covered by definition below the surface).
