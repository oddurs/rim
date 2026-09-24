---
id: 158
title: 'Audit float math in the sim: fixed-point, or f64 with rules?'
type: spike
status: backlog
milestone: co-op
depends_on:
- 157
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: none
effort: m
layer: engine
area: sim
pillar:
- determinism
---

## Question

DESIGN.md §7 says sim math is integer or fixed-point, but needs, wealth, colony strength and rates use `f64` (e.g. `systems.rs`, `World::wealth`). Basic IEEE operations match across x86 and ARM, but any C math library call, or a compiler fusing multiply-add, can split platforms and desync co-op. What's the rule?

## Options

- Convert sim state to fixed-point/integers and keep floats out of `rim_sim` except at the edges
- Keep `f64`, forbid math-library functions in sim code (clippy lint or a wrapper type), and rely on the cross-machine hash check
- Hybrid: integers for stored state, `f64` only in transient calculations whose results are rounded deterministically

## Decision


## Acceptance criteria

- [ ] Inventory of every float in sim state and hot paths
- [ ] Decision recorded in DESIGN.md §7, and the text there matches the code
- [ ] Follow-up items for the conversion or the lint

## 2026-09-23

Weather (0179) is designed fixed point end to end: terms and curves in integer hundredths (0181), stock fields in i16/i32, noise from integer hashes. Needs (systems::needs) still compute field-driven drain in f64; audit that when the warmth need moves to feels_like (0189).
