---
id: 495
uid: 7dcb8a90-e24a-4547-9f2e-2259ee83bf4b
title: A seed per test, the nightly shift, and a one-line repro on every failure
type: feature
status: done
milestone: proving-ground
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
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

- [x] A deliberately failing Rust test and Luau test each print a repro line that, run as printed, fails the same way (shown)
- [x] `t.world()` without a seed no longer means seed 1 (test)
- [x] The suite has been run with a shift; each seed-coupled test found is fixed or has an item (listed here)

## 2026-09-28

Built: rim_sim::testseed. A seed from a name is FNV-1a mixed, capped under 2^53 so a Luau test can read it back with w:seed() and pass it on exactly, XOR RIM_SEED_SHIFT; RIM_SEED pins it. t.world() with no seed takes the seed of 'mod/tests/file.luau/test name'. Every failure from rim test ends with a rerun line: 'seed N (from "...", shift S); rerun: RIM_SEED=N rim test <mod dir> --filter "name"', or just the rerun when the test named its seed. Rust tests get TestSeed::new(), seeded from the test's thread name (nextest runs each test in its own process); if the test panics it prints 'seed N (...); rerun: RIM_SEED=N cargo nextest run --release -p <crate> --test <binary> -E test(=<name>)'. The Rust line has no tick: the guard can't see the sim, and the rerun reproduces it anyway.
Shown: a throwaway Rust test failing under RIM_SEED_SHIFT=20260928 printed its line; run as printed it failed with the same state hash (5321e27773b6b92d). A throwaway Luau test did the same through target/release/rim test (hash d89f6e0e70a25cb2 both times). The test a_world_with_no_seed_gets_one_from_the_test_name checks t.world()'s derived seed, that w:seed() round-trips, and both rerun lines.
With RIM_SEED_SHIFT=20260928 the whole suite (840) and rim test (16) pass: no test derives its seed yet, since every Rust and Luau test names one. Moving the tests whose map doesn't matter is d1f37e29.

## 2026-09-28

The nightly lane's test, sim (mods) and nightly jobs set RIM_SEED_SHIFT to the UTC date (a step before the build); PRs and the queue run with shift 0.
