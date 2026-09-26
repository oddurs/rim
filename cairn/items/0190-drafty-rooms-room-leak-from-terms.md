---
id: 8376a04f-f93e-4daf-9685-183c3dc9cf00
title: 'Drafty rooms: room leak from terms'
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
depends_on:
- 3114946b-5434-4171-9cdb-86ae4e7bb38d
- 3bb54ba3-21f9-42a4-9f7c-8299b5db1db5
created: 2026-09-23
updated: 2026-09-25
closed_at: 2026-09-25
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

- [x] `leak_per_hour` still loads unchanged
- [x] A room cools faster in wind than in calm (test)
- [x] Balance rerun: first week unchanged within the harness's noise

## 2026-09-23

Moved to Colony.

## 2026-09-25

A room field's leak can be terms (field.leak.<label>, the same shape as ambient) instead of the constant leak_per_hour, which still loads (a field gives one or the other). Evaluated once per room step against the outdoor values, in fixed point like every term, so it reads wind, or anything a mod adds, with no engine code. Core: sealed 0.06 plus a wind term, 0.06 × curve(wind: 0 below 4 m/s, 0.8 at 16, 1.0 at 25), so a breeze does nothing and a storm leaks 1.8x. Balance (balance example, 60 seeds x 7 days, weather on): with a fire in the hut, byte-identical to main over 20 seeds (the fire's cap holds the room). Without a fire the runs diverge chaotically, and the aggregates move within noise: lost 2 to 3 of 60, runs with a death 7 to 10, froze 14.5 to 12.3 colonist-hours per run, warmth low after night one 0.08 to 0.06, hut timing unchanged.
