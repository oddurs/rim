---
id: fb27984a-1895-4568-a24d-e0bc8170dc29
title: A tiny positive grow or spoil `days` passes validation and divides by zero
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: engine
area: modding
---

## What

`defs.rs` (~3051 and ~3068) checks `spoil.days` and `grow.days` only as floats: `days > 0.0 && days.is_finite()`. The consumers divide by the fixed-point value:
- grow: `systems.rs` ~451, `rate * days / terms::to_q(gd.days)`
- spoil: `systems.rs` ~53-55, `... / (keeps * TICKS_PER_DAY * days)` with `days = to_q(sp.days)`

A value under half of 1/Q (for example `days = 0.00004`) rounds to 0 in `to_q`. The integer division by zero then panics, in release builds too, on the first grow or spoil pass that touches such a def.

## How it fails

A mod with an absurdly small `days` crashes the game mid-play, instead of failing at load with a message.

## Reproduce

Not yet run; verified by reading. tests/growth.rs `GARDEN` with `days = 0.00004`, then grow for a while. Or tests/spoilage.rs with a patch setting berries' `spoil.days = 0.00001`.

## Fix

Require `to_q(days) >= 1` at load, and say so in the error.

## Acceptance

- [ ] Such a def fails the load with a message
- [ ] A test that fails before the fix and passes after
