---
id: 3163979c-4bcc-4b59-a524-a1f1167cb965
title: 'No wall-clock assertions in the suite: count work, and a guard'
type: chore
status: backlog
milestone: proving-ground
depends_on:
- 6b605f27-ac8f-4f3d-9de0-3967d8b84b32
- e2c56c9a-f105-4081-a5d9-b11635616338
- e8673d3f-d5fb-4922-86ea-d71c521b2a3a
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: none
effort: m
layer: tooling
area: tests
---

## Problem

Seven places in the default suite assert elapsed time: `rim_sim/tests/boundary.rs:258` (6b605f27), `rim_sim/tests/scripting.rs:401` (e8673d3f), `rim_ui/tests/engine.rs` (the frame budget, e2c56c9a), `rim_ui/tests/tokens.rs`, `rim_ui/tests/stores_sheet.rs`, `rim_ui/tests/fonts.rs` and `rim_client/src/draw.rs`. Three have failed CI on a busy runner. A time budget belongs in a bench (DESIGN.md §8a), not in a gating test.

## Proposal

- The three existing items each fix their own test. This item fixes the other four the same way: assert a count (steps, cells visited, draw calls, cache hits against scans) or move the timing to a bench.
- A guard test in `rim_sim` that scans `crates/*/tests/**/*.rs` and `crates/*/src/**/*.rs` test modules for `Instant::now`, `elapsed()` and `SystemTime` and fails with the file and line, with an allowlist for the benches.

## Acceptance criteria

- [ ] None of the seven sites asserts wall-clock time (each listed here with what it counts instead)
- [ ] The guard test fails on a deliberately added `Instant::now()` in a test and passes on main
- [ ] Each rewritten test still fails when the thing it guards regresses (a deliberate regression shown for each)
- [ ] The suite passes three times in a row under `nice` with four parallel builds loading the machine
