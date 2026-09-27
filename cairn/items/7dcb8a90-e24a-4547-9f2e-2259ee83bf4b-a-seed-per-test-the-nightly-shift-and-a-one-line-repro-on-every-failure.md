---
id: 7dcb8a90-e24a-4547-9f2e-2259ee83bf4b
title: A seed per test, the nightly shift, and a one-line repro on every failure
type: feature
status: backlog
milestone: proving-ground
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: additive
effort: s
layer: engine
area: tests
---

## Problem

Tests pick seeds by hand (seed 1 by default in Luau's `t.world()`), so a test can come to rely on what a particular map contains without anyone noticing, and a failing test says little about how to bring it back. DESIGN.md §7b.

## Proposal

- A helper for Rust tests and `t.world()` in Luau: a test that doesn't name a seed gets `hash(test name) ^ RIM_SEED_SHIFT`; the shift is 0 unless the environment sets it (the nightly lane sets the date).
- On failure, both print the seed, how it was made, the tick, and the exact command to rerun the one test with that seed (`RIM_SEED=... cargo nextest run ...` or `rim test --filter ...`).
- Run the suite once with a non-zero shift and file an item for every test that fails only because of it.

## Acceptance criteria

- [ ] A deliberately failing Rust test and Luau test each print a repro line that, run as printed, fails the same way (shown)
- [ ] `t.world()` without a seed no longer means seed 1 (test)
- [ ] The suite has been run with a shift; each seed-coupled test found is fixed or has an item (listed here)
