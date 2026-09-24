---
id: b04a908e-8251-4d3a-a5a8-638c9b1759db
title: 'Stable entity ids: a Uid on every entity, and ties broken by it'
type: feature
status: backlog
milestone: persistence
created: 2026-09-24
updated: 2026-09-24
priority: p0
api: additive
effort: m
layer: engine
area: sim
pillar:
- determinism
---

## Why

The log is the save (DESIGN.md §7a), so a command recorded before a save must
name the same entity after the load. Commands, jobs, reservations and scripts
hold raw hecs handles today (`Command::Draft { pawn: Entity }`, `rim_sim_entity`
in script.rs), and a load can't reproduce hecs's handles or its free list.

Iteration order also leaks into the sim: `find_food` keeps the first-found
best (`b.0 <= d`), so ties go to whatever order hecs iterates in, and a load
won't rebuild the same archetype order.

## What

- A `Uid(u64)` component on every entity, from a counter the world owns.
- Commands and the script API name entities by `Uid`. Jobs and reservations
  keep `Entity` in memory; the save translates them at the boundary.
- Every choice between equals breaks the tie by `Uid`.

## Acceptance criteria

- [ ] Every spawned entity gets a Uid from a world counter, never reused
- [ ] Commands and the Luau API take and return Uids; hecs handles never leave the engine
- [ ] Every "pick the best" loop breaks ties by Uid
- [ ] Test: the determinism scenario gives the same hash when the ECS is rebuilt in reverse spawn order
