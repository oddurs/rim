---
id: 70a9cc25-691d-46a6-a4a1-580addf90d73
title: rim.damage changes a stack's condition behind the stock ledger's back
type: bug
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
layer: engine
area: scripting
---

## What

`rim.damage` (crates/rim_sim/src/script.rs, ~line 1795) writes `Thing::hp` directly when the thing survives. A stack's condition decides whether a store keeps it: `kept_level` goes through `Filter::takes`, and a condition filter reads hp. `World::set_stack_hp` exists to take a stack out of the stock ledger and the store index at its old hp and put it back at its new one. `rim.damage` skips it.

## How it fails

1. A zone with a condition filter (say min 50%) keeps a whole stack, so `Stock::stored` counts it.
2. A script damages the stack below 50%. The ledger still says it's kept, and the store index still has it as sorted.
3. When the stack later leaves, `stock_change` works out `kept = false`, so `stored` is never taken down.

Consequences:
- A debug build fails the ledger assert in `Sim::step` within 1,000 ticks.
- `rim.stock(..., "loose")` computes `on_map - stored` as u32 and underflows. Debug panics; release wraps to about 4e9, and core's readings read it every hour, which drives standing orders.
- A load recounts the ledger, so the loaded game reads different stock from the live one.

No shipped script damages stacks today (fire damages fixtures), so this is reachable only through mods. Found by the script review sweep.

## Reproduce

A zone with a condition filter keeping a stack. A probe script calls `rim.damage` on it, bringing it under the filter's minimum. Then `assert_eq!(world.stock, world.counted_stock())` fails.

## Acceptance

- [ ] `rim.damage` on a surviving stack keeps the stock ledger and store index right
- [ ] A test that fails before the fix and passes after
