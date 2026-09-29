---
id: 362
uid: 6b605f27-ac8f-4f3d-9de0-3967d8b84b32
title: boundary_refresh_is_cheap asserts wall-clock time and fails under load
type: bug
status: done
milestone: proving-ground
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
priority: p0
api: none
effort: s
layer: engine
area: tests
---

## What happens

`tests/boundary.rs::boundary_refresh_is_cheap` asserts the refresh takes under 5 ms of wall-clock time. With three release builds sharing the CPU it failed once, then passed three times alone.

## What should happen

Measure work, not time: count cells or rooms visited, as the other budget tests that were made robust do (see e2c56c9a for the same problem in the UI).

## Reproduction

Seed: 21
Mods: core

1. Load the machine (parallel cargo builds).
2. `cargo test --release -p rim_sim --test boundary`.

## 2026-09-27

Counted, not timed: refresh_boundaries returns how many boundary pieces it looked up. The test asserts that equals the rooms' total boundary (each piece once, whatever the number of fields; 14 rooms here) and that a refresh without a room rebuild does none. A deliberate lookup per field fails it (15290 against the expected count). No clock in the test.
