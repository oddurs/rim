---
id: 181
title: 'Terms and curves: a fixed-point evaluator for outdoor values'
type: feature
status: done
milestone: weather
depends_on:
- 180
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
priority: p0
api: additive
effort: s
layer: engine
area: modding
pillar:
- plugin-first
- determinism
- performance
---

## Why

Outdoor values (temperature, light) are computed by a Luau script today, and the last writer silently wins. The same shape recurs everywhere in weather: a sum of labelled terms, each a scale times a product of inputs through curves (DESIGN.md §4c). One small evaluator makes these values data a mod can patch, deterministic, and able to explain themselves.

## What

- Terms are tables keyed by label, so a patch can change one term: `[field.ambient.day] scale = 9, of = [{ input = "hour", curve = [...] }]`.
- v1 inputs are global: `year` (0–1), `hour` (0–24), `ambient = id`, `noise = key` (smooth deterministic noise, `hours` period), and numbers. Per-cell inputs come with stock fields (0186).
- Compiled at load: resolved ids, fixed-point breakpoints (up to 16). Unknown inputs, ids and unsorted curves are load errors naming the mod, def and term.
- Evaluated in integer fixed point, no floats. `explain` returns each label's contribution.

## Acceptance criteria

- [x] Terms parse with errors naming the def and term label
- [x] Integer evaluation matches an f64 reference within 0.01 (property test)
- [x] `explain` contributions sum to the value
- [x] A patch changes one term by label

## 2026-09-23

crates/rim_sim/src/terms.rs. Terms are tables keyed by label (`[field.ambient.day]`), so the loader's deep merge patches one term and reports conflicts per term; `scale = 0` switches one off. Inputs: input=year|hour, ambient, noise (hash-based value noise, no RNG draws), constants; curves up to 16 points, clamped. Q = 1e4 fixed point with i64 intermediates. Property test: worst error vs an f64 reference < 0.01 over 2000 random curves. Load errors name the def and term ('field/temperature, term 'day': unknown input 'moon''). Tests: crates/rim_sim/tests/climate.rs, weather.rs, weather_guide.rs (release, as CI runs them).
