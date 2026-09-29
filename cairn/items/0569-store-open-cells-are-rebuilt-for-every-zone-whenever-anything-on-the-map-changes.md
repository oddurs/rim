---
id: 569
uid: 13944b55-f172-4c74-b7f2-71efed668727
title: Store open cells are rebuilt for every zone whenever anything on the map changes
type: perf
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
layer: engine
area: sim
---

## What

`World::sync_stores` (world.rs ~2844) runs every tick. It calls `StoreIndex::rebuild_open` (store.rs ~129), which walks every cell of every zone, whenever `map.revision` moved. The revision moves on any fixture, terrain, footing or water-cost change anywhere on any level: building, a tree felled, a plant spreading, water rising.

Separately, `stores_changed` (a container built, destroyed or re-filtered) rebuilds the whole store index, walking and sorting every thing in the world (`rebuild_stores`, world.rs ~2816).

## Why it matters

In an active colony the revision moves nearly every tick, so every zone cell is re-checked every tick, and that cost grows with total stockpile area, not with what changed. Container churn costs O(things log things) each time.

## Direction

Re-check only the zone cells whose cells changed. The map already records changed cells per chunk (`fixture_rev`, `changed`), and zones know their cells. Update the index for one container, rather than rebuilding it.

## Acceptance

- [ ] A change outside every zone costs the store index nothing
- [ ] A container added or removed updates the index for that container only
- [ ] Tests compare the incremental index with a rebuild (`StoreIndex` is `PartialEq` for this)
