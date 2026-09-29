---
id: 365
uid: 6fd6b13b-1186-46f4-876e-743d173e03d7
title: Firelight baked with soft shadows into four flicker channels
type: feature
status: done
milestone: lighting
assignee: Oddur Sigurdsson
depends_on:
- 421
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] Bench: steady-frame cost with 10 and with 500 static lights differs by less than noise
- [x] Bake time for the dusk scene recorded here
- [x] Two adjacent torches don't pulse in step (test on channel assignment)
- [x] Api: `glow` documented in the defs reference

## 2026-09-26

Measured (release, 250x250 dusk, Apple M4 Pro under load, CPU only): per-frame lighting CPU is 0.017 ms with 40 lamps and 0.015 ms with about 500 (three per room, local measurement only). The firelight pass did work on 0% of steady frames in both cases; the multiply is 0.007-0.009 ms either way. A bake costs 0.2-0.9 ms of CPU (noisy) to list the lights and build one mesh; its GPU time can't be taken on this GPU. Rebakes only when the light emitters themselves change (place, strength, reach), or when a wall changes within a light's reach, using the occluders' changed chunks. A heater re-stamping or a wall far from any fire doesn't rebake. An earlier version keyed on the field revision, which moves for every emitter; the extra rebakes slowed frames enough to shift the autotest's world, which the client advances by real frame time. Found in testing: macroquad passes vertex colours as bytes, so the bake's channel mask needs /255; without it every light saturated to its reach.

## 2026-09-27

Review fixes: a mesh holds 4,000 lights, macroquad's draw cap, not 16,000. Reach is capped at the 23 cells the trace covers. Radius-0 emitters still light their own cell. A cell now passes exactly 1 - opacity whatever the angle, so a window passes its 35%. The flame's own cell is skipped, so a light set in a wall isn't shaded by it. The 0.6 storage scale is one constant, passed to both shaders, and strength is clamped to what it can hold. emitters_of uses FIXED. Rebakes are partial: a light that comes, goes or changes, or a wall near a light, redoes only that light's area, cleared and redrawn under a scissor from every light reaching it. A light indoors redoes everything, since its room's fill changes. The autotest checks a partial bake against a whole one (they match exactly) and that the new fire took 2 draws, not the map. The docs now say diagonal fires can pulse together. Re-measured with exactly 10 lights (the review was right that I had used 40): per-frame lighting 0.035 ms with 10, 0.016 ms with about 500; noise, since the 10-light run was slower overall. The firelight pass did no work on steady frames in either. Criterion 2 is unticked until a GPU bake time exists; this Mac can't time a pass, so it comes from CI's bench.

## 2026-09-27

Bake time, from CI's bench on #233 (07eb86bf, Linux llvmpipe, software GL): a whole firelight rebake of the 250x250 dusk scene takes 20.0 ms of GPU and 0.21 ms of CPU; the sun pass 16.2 ms of GPU. That is a software rasteriser running every fragment on the CPU, so it is an upper bound, and it is paid only on the frame an edit lands: steady frames ran no firelight pass in any view. A partial rebake (a light or a wall near one) redraws only that light's area. A real GPU's number still needs one that can time a pass; this Mac can't.
