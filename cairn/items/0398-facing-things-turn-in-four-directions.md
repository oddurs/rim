---
id: 398
uid: ba18a8e4-6893-4e2e-8e30-9d8040b41109
title: 'Facing: things turn in four directions'
type: feature
status: done
milestone: houses
assignee: Oddur Sigurdsson
depends_on:
- 290
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] A bed built facing east covers the two cells east–west, and its spot turns (test)
- [x] Saves round-trip facing; an old save loads everything facing south
- [x] The determinism test passes

## 2026-09-26

Facing is a field on Thing (serde default 0, not written when 0, so old saves and hashes don't move). ThingDef::footprint(at, facing) is the one place a footprint turns, built on offset so it keeps the level for depth; ThingDef::turn turns spot offsets the same way. The client turns a look's layers over the footprint (Orient.turn) and derives a chair's facing from look.face = "beside:table". T turns the build tool (R is draft).
