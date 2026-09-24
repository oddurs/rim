---
id: 183
title: 'Ambient terms: outdoor channels computed from data, with named pushes'
type: feature
status: doing
milestone: weather
depends_on:
- 181
- 182
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: additive
effort: m
layer: engine
area: sim
pillar:
- plugin-first
- determinism
---

## Why

Declaring outdoor values as terms makes the climate data, lets incidents and plugins add to it instead of overwriting it, and gives every value a breakdown.

## What

- `[[field]]` gains `ambient = { label = term, ... }`. Fields evaluate in dependency order every 20 ticks; a cycle is a load error.
- `rim.push_ambient(field, key, value, hours?, ease_hours?)`: a named contribution added on top of the terms, easing from its current value over `ease_hours` and expiring after `hours`. `rim.clear_ambient(field, key)`.
- `rim.set_ambient` pins a value (tests and tools), overriding terms and pushes; `nil` unpins.
- Core fields (shared vocabulary): `temperature`, `light`, `cloud`, `precipitation`, `wind`, `wind_dir`, `fog`, with no weather of their own. `20_climate.luau` is deleted; day and night become core data.
- `rim.explain(field)` and `view.explain(field)`: each term and push with its contribution.

## Acceptance criteria

- [ ] Core's day and night are data; `20_climate.luau` is gone
- [ ] With core alone, each hour of the first three days is within 1.5°C of today's curve
- [ ] Pushes add, ease, expire and show in the breakdown
- [ ] Dependency cycles are reported at load
- [ ] Tick cost measured before and after
