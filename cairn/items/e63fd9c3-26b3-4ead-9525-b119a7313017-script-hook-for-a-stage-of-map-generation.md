---
id: e63fd9c3-26b3-4ead-9525-b119a7313017
title: Script hook for a stage of map generation
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-24
updated: 2026-09-24
priority: p3
api: additive
effort: m
layer: engine
area: scripting
pillar:
- plugin-first
- determinism
---

## Why

Terrain bands, spawns and wildlife are data, but the stages that combine
them are Rust. DESIGN.md §6a says generation is a stage, not a secret: a
mod that wants rivers, ruins or a hand-laid start should be able to take
over one stage without a fork.

## What

- Generation split into named stages (terrain, fixtures, start, wildlife).
- `rim.on_generate(stage, fn)`: a script runs after, or instead of, the
  engine's stage, writing through `rim.set_terrain` and spawns.
- The engine's noise exposed to scripts. Today `fbm` runs in `f64`; it
  moves to fixed point first so a Luau-generated map matches across
  platforms (§7).

## Acceptance criteria

- [ ] The example plugin replaces one stage and the determinism test passes
- [ ] A script cannot write terrain outside generation
- [ ] Noise in fixed point, with the core map visually unchanged (screenshots)
