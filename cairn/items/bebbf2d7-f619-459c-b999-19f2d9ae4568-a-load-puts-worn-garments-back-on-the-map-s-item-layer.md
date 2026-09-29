---
id: bebbf2d7-f619-459c-b999-19f2d9ae4568
title: A load puts worn garments back on the map's item layer
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: none
layer: engine
area: save
---

## What

`Snapshot::restore` (crates/rim_sim/src/snapshot.rs, the "map's entity layers" pass, ~line 844) lays every `Thing` back on the map except those `Held`, `Contained` or `Replaces`. It doesn't exclude `Worn`. `World::put_on` takes a garment off the item layer but leaves its `Thing.pos` where it lay, so on load the garment someone is wearing goes back on the item layer at that cell, and its emitters are re-added there.

## How it fails

- The loaded game differs from the one that kept running (the item layer, stock and store index aren't in `state_hash`, so the hash doesn't notice):
  - `is_stack` is true for the worn garment, so `recount_stock` counts it and `rebuild_stores` can list it as unsorted: a hauler can pick up a garment a colonist is wearing.
  - If a stack has since been dropped on that cell, whichever has the higher id wins the cell and the other is left off the map.
- Found by the save/load review sweep (Bare Metal engine review).

## Reproduce

In `tests/apparel.rs`, after a colonist puts a garment on and the game is saved and restored: `back.world.map.item_at(where_it_lay)` is the garment, and `back.world.stock != s.world.stock`.

## Acceptance

- [x] A worn garment is on no map layer after a load
- [x] The loaded stock ledger equals the live one
- [x] A test that fails before the fix and passes after
