---
id: 6b605f27-ac8f-4f3d-9de0-3967d8b84b32
title: boundary_refresh_is_cheap asserts wall-clock time and fails under load
type: bug
status: backlog
milestone: scale
created: 2026-09-26
updated: 2026-09-26
priority: p3
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
