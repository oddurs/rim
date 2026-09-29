---
id: 619
uid: 512b9f1a-55cc-4082-b945-7d7c86f2f942
title: Script numbers overflow engine arithmetic in several bindings
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: engine
area: scripting
---

## What

Each of these panics in a debug build and misbehaves in release:
- `rim.random_int(0, 2147483647)`: rng.rs ~35 `(hi - lo + 1)` overflows; in release, `(i32::MIN, i32::MAX)` always returns `lo`.
- `rim.near_cell(x, y, i32::MIN)`: `-r` overflows (script.rs ~1147); a negative `r` always offsets by (+|r|, +|r|).
- `rim.push_ambient(f, k, v, math.huge)`: `ticks(inf)` is `u64::MAX`, and `tick + …` overflows (field.rs ~577). In release, a push meant to last forever expires at once. `Push::value` has the same problem with `start + ease` (~62).
- `rim.field_add(..., math.huge)`: `*cell as i64 + to_q(v)` overflows (field.rs ~799).
- `rim.leave_after(id, 2^64 - 4096)`: overflows at script.rs ~1705.

## Reproduce

Not yet run; verified by reading. The tests/scripting.rs `run()` helper, with each call in a hook: a debug `cargo test` panics.

## Fix

Saturating arithmetic, or a script error for arguments out of range.

## Acceptance

- [ ] None of these panics or wraps; each clamps or is a script error
- [ ] A test that fails before the fix and passes after
