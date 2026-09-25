---
id: b04a908e-8251-4d3a-a5a8-638c9b1759db
title: 'Stable entity ids: the world hands them out, and ties are broken by them'
type: feature
status: done
milestone: persistence
assignee: Oddur Sigurdsson
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
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

- The world allocates every entity id from a counter it owns and never
  reuses one, spawning with hecs's `spawn_at`. The hecs handle is then the
  stable id, so commands, jobs, reservations and scripts need no change.
- Every choice between equals breaks the tie by id.

## Acceptance criteria

- [x] Every spawned entity gets its id from a world counter, never reused
- [x] Commands and the Luau API name entities by that id
- [x] Every "pick the best" loop breaks ties by id
- [x] Test: the determinism scenario gives the same hash when the ECS is rebuilt in reverse spawn order

## 2026-09-24

No Uid component after all: hecs's spawn_at lets the world pick each handle, so World::spawn hands out ids from its own counter (generation always 1) and never reuses one. The handle is the stable id, and commands, jobs, reservations and the Luau API (to_bits) needed no change: no lookup map, no translation. Cost: hecs keeps ~16 bytes of metadata per id ever used (meta entry plus its free list), which grows with every spawn; tens of thousands a game-year is well under a megabyte. Ties: door, food, spots, work, hunt and nearest_item now compare (key, id); wealth sums in id order because float addition isn't associative. Without those, reversing hecs's order at tick 100 diverged seeds 1, 2, 3 and 5 within 4,000 ticks (seed 3 at tick 325); the test pins seeds 3 and 5. For the snapshot (c5d185be): the counter must be saved, pawns restored in id order, and field emitters (a Vec in insertion order) rebuilt in id order.

## 2026-09-24

Review: the hunt loop's pre-check ignored the id; fixed. Noted for c5d185be: next_entity needs saving (it's private, so the loader lives in world.rs or gets an accessor), and the loader should spawn in ascending id order, since hecs's alloc_at scans its free list linearly for any id below meta.len().
