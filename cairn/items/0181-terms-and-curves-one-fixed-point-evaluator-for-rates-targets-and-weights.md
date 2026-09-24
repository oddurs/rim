---
id: 181
title: 'Terms and curves: one fixed-point evaluator for rates, targets and weights'
type: feature
status: planned
milestone: weather
depends_on:
- 180
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: additive
effort: m
layer: engine
area: modding
pillar:
- plugin-first
- determinism
- performance
---

## Why

Every rate, target and weight in the weather system is the same shape: a sum of labelled terms, each a scale times a product of inputs through curves (DESIGN.md §4c). One evaluator, written once and fast, means every part of weather is data a mod can patch, and every value can explain itself.

## What

- Parse `[{ label, scale, of = [input, ...] }]` wherever a def accepts terms. Inputs: `ambient`, `field`, `self`, `base`, `above_base`, `terrain`, `sky`, `near`, `year`, `hour`, `weather`, `noise`, constants; each optionally `curve = [[x, y], ...]`.
- Compile at load into a flat, allocation-free program: resolved field indices, fixed-point breakpoints, input kinds as an enum. Unknown inputs, unknown ids, unsorted curves and more than 8 points are load errors naming the mod and the def.
- Evaluate in integer fixed point (i64 intermediates, hundredths in and out). No floats, no transcendental maths.
- `explain`: the same evaluation returning each label's contribution, for devtools, the HUD and tests.
- `noise = key`: smooth value noise over ticks from a hash of (seed, key, period), deterministic without drawing from the world RNG.
- Patches can replace a whole term list or one term by label (ties into 0144 list operations).

## Acceptance criteria

- [ ] Terms parse from TOML with errors naming the mod, def and term label
- [ ] Evaluation is integer-only and matches an f64 reference within 0.01 over randomised curves (property test)
- [ ] `explain` returns per-label contributions that sum to the value
- [ ] A patch can replace one term by label without restating the list
- [ ] Bench: at most 20 ns per term per cell in release, recorded here
