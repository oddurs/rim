---
id: a6304f70-ee45-48fd-8282-cdb59aa00f70
title: Noise `hours` of inf or huge passes validation and overflows terms
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

terms.rs ~455 rejects only `hours <= 0.0`. At ~458 `.round().max(1.0) as u64` saturates `inf` (or ≥ about 1.1e16) to `u64::MAX`. `/ period as i64` then divides by -1 (~470), and `f * f` overflows (~474) once the tick passes about 3e5. `nan` silently becomes period 1.

## How it fails

A debug build panics about 15 game days in; release computes garbage.

## Reproduce

Not yet run; verified by reading. Extend the bad-input table in terms.rs `a_cycle_runs_zero_to_one…` with `{ noise = "k", hours = inf }` and expect `Err`.

## Fix

Bound noise `hours` as the cycle input's range is bounded (`1/24..=3.65e8`), and reject non-finite values.

## Acceptance

- [ ] Out-of-range or non-finite noise hours fail the load
- [ ] A test that fails before the fix and passes after
