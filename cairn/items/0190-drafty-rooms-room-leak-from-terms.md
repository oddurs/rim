---
id: 190
title: 'Drafty rooms: room leak from terms'
type: feature
status: backlog
milestone: colony
depends_on:
- 181
- 183
created: 2026-09-23
updated: 2026-09-23
priority: p3
api: additive
effort: s
layer: engine
area: sim
pillar:
- survival
---

## Why

A hut loses heat faster in a gale. Letting a room field's leak be terms means wind, and later doors left open or better walls, change insulation without new engine code.

## What

- `leak = [terms]` on room-state fields, replacing the constant `leak_per_hour` (still accepted as a single constant term).
- Core: leak rises with wind speed (×1.8 in a storm).

## Acceptance criteria

- [ ] `leak_per_hour` still loads unchanged
- [ ] A room cools faster in wind than in calm (test)
- [ ] Balance rerun: first week unchanged within the harness's noise

## 2026-09-23

Moved to Colony.
