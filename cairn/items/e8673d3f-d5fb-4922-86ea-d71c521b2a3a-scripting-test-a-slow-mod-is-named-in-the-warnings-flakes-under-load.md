---
id: e8673d3f-d5fb-4922-86ea-d71c521b2a3a
title: scripting test a_slow_mod_is_named_in_the_warnings flakes under load
type: bug
status: backlog
milestone: scale
created: 2026-09-25
updated: 2026-09-25
priority: p2
api: none
effort: s
layer: engine
area: tests
---

## Why

`crates/rim_sim/tests/scripting.rs` `a_slow_mod_is_named_in_the_warnings` asserts on wall-clock profiler warnings. It failed once in a full `cargo test --release --workspace` run on a loaded machine, then passed three times alone. A wall-clock threshold makes CI red for reasons unrelated to the change (#92 fixed the same kind of flake in `rim check`).

## What

- Assert on the probe mod's share of script time, or its step count, not an absolute time.
- Or run it with a threshold scaled by a measured baseline in the same process.

## Acceptance criteria

- [ ] The test passes under a parallel full-suite run on a busy machine
- [ ] It still fails when the probe mod is not slow
