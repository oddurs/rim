---
id: 183
title: 'Ambient terms: outdoor channels computed from data, with named pushes'
type: feature
status: done
milestone: weather
depends_on:
- 181
- 182
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
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
- Core fields (shared vocabulary): `temperature`, `daylight`, `light`, `cloud`, `precipitation`, `wind`, `wind_dir`, `fog`, with no weather of their own. `20_climate.luau` is deleted; day and night become core data.
- The sun is a `daylight` term (`sun`), and `light`'s term is `daylight` times a curve of `cloud`. Sky mods patch `daylight` and weather sets `cloud`, so neither has to name the other (DESIGN.md §4c, "The sky").
- `rim.explain(field)` and `view.explain(field)`: each term and push with its contribution.

## Acceptance criteria

- [x] Core's day and night are data; `20_climate.luau` is gone
- [x] With core alone, each hour of the first three days is within 1.5°C of today's curve
- [x] Pushes add, ease, expire and show in the breakdown
- [x] Dependency cycles are reported at load
- [x] A test mod that replaces core's `sun` with two suns still gets darker as `cloud` rises
- [x] Tick cost measured before and after

## 2026-09-23

Split daylight from light (2026-09-23): a modded sky (two suns, a moon) must not break weather's cloud dimming. If weather patched core's sun term, a mod replacing sun would silently drop it. With light = daylight x curve(cloud), each side owns one field. Follow-up for moons and colour: 0209.

## 2026-09-23

20_climate.luau deleted; core's day is `mean` + `day` terms, the sun is daylight's `sun` term, light = daylight x cloud curve. Worst hourly error vs the old smoothstep script over three days: under 1.5°C (piecewise curve sampled every 3 h). Pushes ease in, add by key, expire by easing out, and list in explain (test); set_ambient pins (overrides terms and pushes, nil unpins) for tests and tools. Cycles fail at load ('a -> b -> a'). Two suns patched onto daylight still dim under cloud (test). Tick cost: headless 5 days, seed 4: mean 0.004-0.005 ms with weather, same as before the sprint (0.004); outdoor values cost O(fields) every 20 ticks. Tests: crates/rim_sim/tests/climate.rs, weather.rs, weather_guide.rs (release, as CI runs them).
