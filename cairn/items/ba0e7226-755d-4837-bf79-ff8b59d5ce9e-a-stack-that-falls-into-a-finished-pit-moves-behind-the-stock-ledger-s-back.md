---
id: ba0e7226-755d-4837-bf79-ff8b59d5ce9e
title: A stack that falls into a finished pit moves behind the stock ledger's back
type: bug
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
layer: engine
area: building
---

## What

`World::dig` (crates/rim_sim/src/world.rs, ~line 1895, the `hole` arm) handles the stack lying on a dig site when the site becomes a hole. It rewrites the stack's `Thing.pos` to the cell below and moves it on the item layer by hand, with no `stock_change`. The stock holdings are kept by chunk, and chunks are per level (`Map::chunk_of`), so the ledger still lists the stack in the surface chunk. Whether a store keeps it changes too, and nothing notes it. If the cell below already holds a stack, the fallen one is set on no layer at all: it still exists, and the ledger still counts it, but nothing can reach it.

## How it fails

- `nearest_stack_where` searches the chunks the holdings name, so the stack below is invisible to material, food and haul searches.
- A debug build fails the "stock ledger drifted" assert in `Sim::step`.
- A load recounts the ledger, so the loaded game finds materials the live one can't: a divergence.

Common in play: a pit or trench dug where something lies, such as the start's items, or a dropped haul. `World::fall` already does this right: it takes the stack up as a lot and places it where it lands.

Found in the engine review.

## Reproduce

Put a stack on open ground, place core's pit there, and `complete_building`. Then `world.stock != world.counted_stock()`.

## Acceptance

- [ ] What lies on a dig site lands below through the ledger, nothing lost
- [ ] A test that fails before the fix and passes after
