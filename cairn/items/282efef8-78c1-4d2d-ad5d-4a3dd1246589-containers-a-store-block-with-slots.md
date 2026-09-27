---
id: 282efef8-78c1-4d2d-ad5d-4a3dd1246589
title: 'Containers: a store block with slots'
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- e953d711-8d06-43a5-b096-4ff7cce491f7
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: additive
effort: l
layer: engine
area: sim
---

## Why

A cell holds one stack and `room_for` is zero on any fixture, so nothing can hold more stacks than the ground. DESIGN.md §4f rules that a container is a fixture whose slots hold `Lot`s by value.

## What

- A `store` block on `[[thing]]`: `slots`, `stack_scale`, `accepts` (what it can ever take, e.g. `not_tags = ["core:bulky"]`), `shelter`, `display` (`fill` | `items` | `none`), `look_stages`.
- Slots hold `Lot`s; the container's cell has nothing in the item layer. Pawns put and take from its `spots`.
- Containers join the store index from the levels item: a filter, a level, room reserved by count.
- Taking from a slot for a bill, a delivery or a meal works like taking from a stack.
- A container that is torn down or destroyed drops its contents nearby (nothing lost).
- Save: an `engine:store` section, FORMAT 5 (agreed with the depth work, which takes 6).
- Scripts: `rim.store(id)`, `rim.store_put`, `rim.store_take`; `item_stored` fires only while a handler is registered.
- Fill stages drive the look; the chunk mesh rebuilds only on a stage change (§6b).

## Acceptance criteria

- [x] A test mod's crate takes four stacks in one cell, and refuses a bulky one (test)
- [x] Bills and building deliveries take from containers (test)
- [x] Tearing down a full crate loses nothing (test)
- [x] Save, load and save again gives the same bytes with containers
- [x] Determinism test passes

## 2026-09-26

Decisions taken on 2026-09-26 when the owner said to build it: core ships zones only, and every container is plugin content. Store levels are five, with their own pips. timber and iron are two plugins. core:wood is relabelled 'logs' by a timber patch; the id stays.

## 2026-09-26

Built with stored stacks as entities (Contained { store, slot } at the container's cell, off the item layer), not Lots in slots as first ruled: the ledger and index had already removed the scan cost that argued for values, and as values every job that takes a thing would need a second kind of source. DESIGN.md §4f keeps both arguments. stack_goal() is the one helper jobs gained. Containers join the store index as StoreKey::Thing; haul reservations for a container are units bound for it. Fill stages (fill_stage) redraw the map only on a stage change; display = items draws up to three slot colours. Loading under different mods fits containers to their defs and sets down contents whose container is gone. item_stored event deferred: nothing listens yet.
