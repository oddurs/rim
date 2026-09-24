---
id: 05dbb688-66e3-47b0-b105-32651a8ebec0
title: 'CI: determinism must hold across machines, not just within one run'
type: chore
status: doing
milestone: shelter
assignee: Oddur Sigurdsson
claimed: 2026-09-24
depends_on:
- 48f92e6e-2187-461a-a98b-f20e36ae1325
created: 2026-09-23
updated: 2026-09-24
priority: p1
api: none
effort: s
layer: tooling
area: tests
pillar:
- determinism
---

## Why

`tests/determinism.rs` compares two runs in the same process, so it can't catch a platform that simulates differently. Co-op and shared replays need macOS, Windows and Linux (x86 and ARM) to reach the same state hash from the same seed and commands (DESIGN.md §7).

## What

Each CI test job runs a fixed scenario (seed, mod set, command script, several in-game days) and uploads its state hash as an artifact. A final job fails if the hashes differ. The hash isn't checked into the repo, so gameplay changes don't need a golden file updated, only agreement between platforms.

## Acceptance criteria

- [ ] Headless runner can print the final state hash for a scenario
- [ ] Every OS in the matrix, including an ARM runner, uploads its hash
- [ ] A compare job fails with every platform's hash listed when they differ
- [ ] The scenario exercises scripts (storyteller, incidents) as well as the engine

## 2026-09-23

Include a year with weather (regimes, stock fields, growth) in the cross-machine hash check once 0184 and 0186 land: it's the most arithmetic-heavy system in the sim.
