---
id: 6fd6b13b-1186-46f4-876e-743d173e03d7
title: Firelight baked with soft shadows into four flicker channels
type: feature
status: backlog
milestone: lighting
depends_on:
- e5d8b445-42ed-4e87-8390-ad54d525757b
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: m
layer: client
area: render
---

## Why

Torches, braziers and campfires don't move, so their shadows never need recomputing. Baked once, they can have far better shadows than a per-frame budget allows, and flicker becomes a few uniforms. DESIGN.md §6e.

## What

- Every static emitter of `light` draws a quad into the static target: falloff `I · (1 − (d/r)⁴)² / (1 + 0.08·d²)`, 8 rays across the flame for soft shadows, walls and closed doors blocking, windows letting light out.
- Four channels: three fire phases and one steady. A light takes a channel by colour and flicker profile; neighbours land in different phases.
- Emitter defs gain an optional `glow = { colour, flicker, height }`. A def that says nothing is a fire.
- Flicker: two octaves of smooth noise with rare gusts, redder when dimmer. Evaluated on the CPU, four per frame.
- Rebake when `fields.revision` changes, only for lights within reach of the change (the sim's re-stamp rule), and per dirty chunk once 96d2dac9 lands.

## Acceptance criteria

- [ ] Bench: steady-frame cost with 10 and with 500 static lights differs by less than noise
- [ ] Bake time for the dusk scene recorded here
- [ ] Two adjacent torches don't pulse in step (test on channel assignment)
- [ ] Api: `glow` documented in the defs reference
