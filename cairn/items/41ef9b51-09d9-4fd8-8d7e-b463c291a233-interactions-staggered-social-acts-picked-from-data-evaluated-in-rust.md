---
id: 41ef9b51-09d9-4fd8-8d7e-b463c291a233
title: 'Interactions: staggered social acts picked from data, evaluated in Rust'
type: feature
status: backlog
milestone: story
depends_on:
- 39915ec5-4ff9-40a7-af6b-aa6d5dbd39a0
- 801c8f78-76af-4aa0-9394-23388a7d1d3c
- bb171d6e-0759-4c87-b9a6-b1df7ee2ca26
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: l
layer: engine
area: ai
pillar:
- plugin-first
- performance
---

## Why

Speech acts are the stuff of relationships. Picking them in Luau per pawn would
cost too much; picking them from data in Rust costs almost nothing (DESIGN.md
§4g).

## What

An awake, unbusy pawn considers an interaction about once an in-game hour,
staggered by id. The partner comes from the pawn index; the act is a weighted
pick over `[[interaction]]` defs whose weights are curves over opinion,
emotion, bond and traits. Effects are data: a relation reason, a thought, a
memory. The act is an event with an intent for the writer. One optional hook
per def may veto, within the mod's budget.

## Acceptance criteria

- [ ] `[[interaction]]` defs: requirements, weight curves, effects
- [ ] Cadence staggered by id; never per tick per pawn
- [ ] The event carries an intent: act, speaker, listener, topic, facts, feelings
- [ ] Veto hook budgeted per mod and profiled
- [ ] Deterministic across machines (crosscheck covers a social soak)
