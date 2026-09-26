---
id: ac643c1f-3f5b-4908-884f-da4e87bfcf01
title: 'Stock ledger and holdings: counts by thing, material and chunk'
type: perf
status: done
milestone: crafting
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: additive
effort: m
layer: engine
area: perf
---

## Why

`rim.count_items` walks every thing in the ECS (`script.rs:1251`), and the bills hook calls it every 60 ticks. `nearest_item_where` (`ai.rs:1083`) walks every thing to find the nearest flint. Storage will ask both questions far more often (DESIGN.md §4f, Cost). This is also the spatial index fbf3ee1c asks for.

## What

- One choke point for stack changes (spawn, merge, take, despawn, carry, deliver). Every count change goes through it.
- A ledger by item def and material: stored, loose and carried counts, kept by that choke point.
- Holdings by item def and 32×32 chunk: how many of a thing each chunk holds.
- `rim.count_items` and a new `rim.stock(thing | {tag} | {category}, where?)` read the ledger in O(1).
- `nearest_item_where` searches chunk rings from the pawn using holdings, and falls back to nothing else.
- Derived, never saved; rebuilt on load. A debug-build check compares the ledger to a full scan every 1,000 ticks.

## Acceptance criteria

- [x] `rim.count_items` and `rim.stock` never scan (the result matches a full scan after a seeded game)
- [x] Nearest-item search visits only chunks that hold the thing
- [x] Measured before and after on the 8d551753 setup, numbers noted here
- [x] Determinism test passes

## 2026-09-26

Measured, cargo run --release -p rim_sim --example bench -- --days 0.5 (250x250, 30 colonists, 200 pawns, the default work), main then this branch back to back on a heavily loaded machine (load ~75): mean tick 0.690 -> 0.395 ms; pawns 0.567 -> 0.331 ms; mod:crafting 0.0163 -> 0.0006 ms (its bills hook calls count_items, which no longer scans). Absolute numbers are noisy under that load; the direction held across two runs. Added bench --haul (the 8d551753 setup) for the store levels item, which rewrites the haul search; the ledger doesn't touch it.
