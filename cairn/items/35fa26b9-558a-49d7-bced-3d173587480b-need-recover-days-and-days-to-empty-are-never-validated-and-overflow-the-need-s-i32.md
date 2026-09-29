---
id: 35fa26b9-558a-49d7-bced-3d173587480b
title: need `recover_days` and `days_to_empty` are never validated and overflow the need's i32
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
layer: engine
area: needs
---

## What

`DefDb::finalize` (defs.rs ~2696-2715) checks a need's `say` and `wet`, but never `days_to_empty`, `recover_days` or `seek_below`. `systems::needs` divides by them (`systems.rs` ~128-132), and adds the rounded delta to the need's i32 (`~137`).

## How it fails

- `recover_days = 0` (or anything below about 1.4e-8): delta = +inf. `Rng::round(inf)` = `i32::MAX`, and `n.1 + i32::MAX` overflows. Debug panics. Release wraps negative and clamps to 0, so the need empties the moment the pawn is comfortable.
- `days_to_empty = 0`, `-0.0` or a tiny negative: the same, the other way.

The same "cast a huge float, then add" pattern is in `ai.rs` ~1995 (food `nutrition` above about 2e5, or inf), `ai.rs` ~2136 (`bed.rest_rate`), and `ai.rs` ~1539 (`regrow_days = inf`: `tick + u64::MAX` overflows, and in release it regrows at once).

## Reproduce

Not yet run; verified by reading. A tests/feels_like.rs-style sim patching `need/core:warmth` with `recover_days = 0.0`, comfortable temperature, then `systems::needs`: debug panics.

## Fix

Validate at load: `days_to_empty` and `recover_days` finite and above 0, `seek_below` in 0..=1, `nutrition`/`rest_rate` finite and in a sane range, `regrow_days` finite.

## Acceptance

- [ ] Each of these fails the load with a message
- [ ] A test that fails before the fix and passes after
