---
id: ba18a8e4-6893-4e2e-8e30-9d8040b41109
title: 'Facing: things turn in four directions'
type: feature
status: backlog
milestone: houses
depends_on:
- f607a83d-1e4b-487c-ab55-7cd58b821603
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: additive
effort: m
layer: engine
area: building
---

## Why

A 1×2 bed, or a workbench with a place to stand in front, can only face one way. Multi-cell things exist (8cf4db07) but can't turn. DESIGN.md §6c rules that things get a facing, and joined pieces derive theirs.

## What

- `Thing.facing` (N, E, S, W), saved. `Command::Build` carries it.
- The footprint, `spots` and look all turn with it. Joined pieces ignore it and take theirs from the run.
- A look can derive its facing: `face = "beside:table"` turns a chair to its table.
- The client uses `R` to turn the plan under the cursor.

## Acceptance criteria

- [ ] A bed built facing east covers the two cells east–west, and its spot turns (test)
- [ ] Saves round-trip facing; an old save loads everything facing south
- [ ] The determinism test passes
