---
id: 39915ec5-4ff9-40a7-af6b-aa6d5dbd39a0
title: 'Traits: pawns carry trait ids; core declares the kind and the founder'
type: feature
status: backlog
milestone: story
depends_on:
- 97e9d4a7-a81b-4107-8ae2-2e4b97b948ab
- fac45b63-b4a1-43b8-a66a-6db8398d4254
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: m
layer: engine
area: sim
pillar:
- plugin-first
---

## Why

Appraisal, voices, grief styles and premises all read traits, and DESIGN.md §3
already relies on a Founder trait that doesn't exist yet.

## Acceptance criteria

- [ ] A pawn carries a list of trait ids; core declares `[[trait]]` and `core:founder`
- [ ] Traits modify stats through the stat pipeline
- [ ] Traits readable from `rim.pawn` and shown in the inspector
- [ ] Settlers roll traits from weights in data, with the world RNG
