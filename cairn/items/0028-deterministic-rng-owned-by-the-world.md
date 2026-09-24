---
id: 6a2574b0-df58-43db-9666-6ab313bdef4c
title: Deterministic RNG owned by the world
type: feature
status: done
milestone: foundations
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: none
effort: s
layer: engine
area: sim
pillar:
- determinism
---

## Why

One seeded RNG, drawn in a fixed order, used by engine and scripts alike.

## Acceptance criteria

- [x] SplitMix-based RNG with range, chance and stochastic rounding
- [x] Stateless coordinate hash for map gen
