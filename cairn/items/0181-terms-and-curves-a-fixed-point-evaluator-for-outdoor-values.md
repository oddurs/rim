---
id: 181
title: 'Terms and curves: a fixed-point evaluator for outdoor values'
type: feature
status: planned
milestone: weather
depends_on:
- 180
created: 2026-09-23
updated: 2026-09-23
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

- [ ] Terms parse with errors naming the def and term label
- [ ] Integer evaluation matches an f64 reference within 0.01 (property test)
- [ ] `explain` contributions sum to the value
- [ ] A patch changes one term by label
