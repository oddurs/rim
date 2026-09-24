---
id: 180
title: 'Weather spike: term evaluator cost and a headless year'
type: spike
status: planned
milestone: weather
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: none
effort: s
layer: engine
area: perf
pillar:
- performance
- determinism
---

## Question

Can a data-declared term evaluator (sums of products of piecewise-linear curves, fixed point) run inside the per-cell loop cheaply enough for stock fields at 250×250, and does a 60-day year with a handful of regimes produce seasons that feel right? Everything else in the sprint builds on the answer.

## Options

- Linear scan over ≤ 8 fixed-point breakpoints per curve, evaluated per cell
- Curves baked into 256-entry lookup tables over each input's declared range (faster, coarser)
- Keep per-cell rules in Rust per field kind, with only coefficients in data (fallback: fast, but a new rule needs an engine change)

## Method

- Prototype the evaluator in `rim_sim` behind a bench (`cargo bench` or an example): 62,500 cells × a 3-term wetness rate with 4 inputs.
- Sketch the core year in a headless example: temperature season and daily curves, 6 regimes, wetness and snow. Print a daily table.
- Check integer interpolation against an f64 reference.

## Acceptance criteria

- [ ] ns per term per cell measured for both evaluator options (numbers recorded here)
- [ ] A 60-day year printed headless: daily mean/min/max temperature, regime, rain, mean wetness, snow
- [ ] Fixed-point error against the f64 reference measured, and within 0.01 of the field's unit
- [ ] Decision recorded in DESIGN.md §4c
