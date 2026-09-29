---
id: c0042c1f-dc72-425a-8578-6f593ff38910
title: A load reads the ground below the surface as 0 until the next ambient update
type: bug
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
layer: engine
area: save
---

## What

`Fields::update_ambient` (crates/rim_sim/src/field.rs, ~line 525) works out each field's `Layer::below`, the outdoor value on each level below the surface from the field's `below` terms. It runs only when `tick % AMBIENT_INTERVAL (20) == 0`. `SavedFields` doesn't keep `below`, and `Fields::restore` doesn't rebuild it. So after a load it is empty, and `Fields::outdoor(f, z < 0)` returns 0 until the next multiple of 20, where the live game has core's ground temperature (10° and up).

## How it fails

The zero reaches the following, and the loaded game diverges from the one that kept running:
- `value_fixed` for unenclosed cells underground
- `carry_rooms_over`: newly dug cells average in 0°, and that persists
- derived fields and plant terms below the surface
- the needs of any pawn underground

Any save at a tick that isn't a multiple of 20 is affected, which is most of them. Found by the save/load review sweep.

## Reproduce

A world with levels (tests/depth.rs): step to a tick with `tick % 20 != 0`, capture and restore, then compare `fields.outdoor(temperature, -1)` live against loaded.

## Acceptance

- [ ] The loaded game's below-surface outdoor values equal the live game's at any save tick
- [ ] A test that fails before the fix and passes after
