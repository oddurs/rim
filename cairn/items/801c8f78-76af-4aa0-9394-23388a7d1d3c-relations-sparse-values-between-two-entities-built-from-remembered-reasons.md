---
id: 801c8f78-76af-4aa0-9394-23388a7d1d3c
title: 'Relations: sparse values between two entities, built from remembered reasons'
type: feature
status: backlog
milestone: story
depends_on:
- 17009725-a061-4445-829f-b1c73c6ce2bd
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: l
layer: engine
area: sim
pillar:
- plugin-first
- performance
---

## Why

Opinion is the first user; goodwill toward a faction and an animal's bond are
the next. Each is a value between two entities that must say why it is what it
is (DESIGN.md §4g, rich or legible).

## What

A mod declares a relation kind with a decay. A value is the sum of reasons;
each reason names an amount, a memory record and a decay. A pair exists only
while it has a reason. Stored in a sorted map for deterministic order.

## Acceptance criteria

- [ ] `[[relation]]` kinds declared by mods, with decay rates
- [ ] `rim.relation(kind, a, b)` and `rim.reasons(kind, a, b)`; `rim.add_reason(...)`
- [ ] Decay in one daily batch; pairs with no reason are dropped
- [ ] Reasons per pair capped, keeping the largest
- [ ] Saved as its own section; bench at 200 pawns shows upkeep per tick
