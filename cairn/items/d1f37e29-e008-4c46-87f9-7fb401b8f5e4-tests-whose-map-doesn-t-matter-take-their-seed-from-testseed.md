---
id: d1f37e29-e008-4c46-87f9-7fb401b8f5e4
title: Tests whose map doesn't matter take their seed from TestSeed
type: chore
status: backlog
milestone: proving-ground
depends_on:
- 7dcb8a90-e24a-4547-9f2e-2259ee83bf4b
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
effort: m
layer: tooling
area: tests
---

## Problem

The seed-per-test machinery (7dcb8a90) is in, but no test uses it yet: every Rust test passes a seed to `Sim::new`/`Sim::build` (84 use 1, 64 use 3, 22 use 21 on 2026-09-28), and every shipped Luau test names one. So the nightly shift can't catch a test that quietly depends on its map.

## Proposal

- Sort the tests: one that sets up what it needs (spawns, digs, a scene) takes `TestSeed::new()` (Rust) or `t.world()` with no seed (Luau). One that needs a particular map keeps its seed with a comment saying why.
- Move them a file at a time, running each file with a few shifts (`RIM_SEED_SHIFT=1`, `2`, `3`). A test that fails under a shift gets fixed (made into a scene) or keeps its seed with a note.

## Acceptance criteria

- [ ] Every `Sim::new`/`Sim::build` in a test either uses `TestSeed` or has a comment saying why its map matters
- [ ] The suite passes with three different shifts (runs linked)
