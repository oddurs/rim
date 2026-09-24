---
id: 209
title: 'Sky bodies: a cycle input and coloured sky terms'
type: feature
status: backlog
milestone: plugin-api
depends_on:
- 56
- 183
created: 2026-09-23
updated: 2026-09-23
priority: p2
effort: m
layer: engine
area: modding
api: additive
pillar:
- plugin-first
---

## Why

The game is hackable, so its sky should be too. After the Weather sprint, a mod can already add a second sun as a `daylight` term (DESIGN.md §4c, "The sky"). Two things are still out of reach. A moon with phases, or an eclipse every twenty days, needs a cycle other than the day and the year. And a green moon is a colour, but the sky tint is one colour curve that a mod can only replace.

## What

- A term input `input = "cycle", days = N` (0–1 phase over N days, with an optional `offset`), in the same fixed point as `hour` and `year`. An eclipse is the product of two cycles through a curve.
- The sky tint becomes labelled colour terms, each a colour weighted by a curve over the same inputs, and the renderer mixes them. Core's dawn, dusk and night are three of them.
- Light stays a scalar in the sim; colour belongs to the renderer and never reaches the state hash.
- An example mod in the modding guide: two suns and a green moon with phases, core's sun replaced, and weather still dimming the light under cloud.

## Acceptance criteria

- [ ] `cycle` evaluates in fixed point and matches an f64 reference within 0.01
- [ ] A mod adds a coloured sky term without replacing core's tint
- [ ] The two-suns example mod loads alongside `mods/weather`, and clouds dim its light (screenshots reviewed)
