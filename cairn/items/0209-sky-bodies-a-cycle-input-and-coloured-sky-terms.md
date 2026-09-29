---
id: 209
uid: ebb814ad-369d-4ddd-99de-e755c2b14eb9
title: 'Sky bodies: a cycle input and coloured sky terms'
type: feature
status: done
milestone: plugin-api
assignee: Oddur Sigurdsson
depends_on:
- 56
- 183
created: 2026-09-23
updated: 2026-09-28
closed_at: 2026-09-28
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

- [x] `cycle` evaluates in fixed point and matches an f64 reference within 0.01
- [x] A mod adds a coloured sky term without replacing core's tint
- [x] The two-suns example mod loads alongside `mods/weather`, and clouds dim its light (screenshots reviewed)

## 2026-09-26

The renderer side of sky bodies (path, colour, softness, shadows) is 1a17d685 in the lighting milestone. This item stays sim-side: the cycle input and coloured terms.

## 2026-09-28

Done as: input = "cycle" with days and an optional offset (days into it at tick 0). It is the tick modulo the period, times Q, over the period, in u128, so it's exact integer maths and draws nothing random. Tested against an f64 reference over 20,000 ticks, including ticks past 2^40: within 0.01. An eclipse (two cycles through a curve) evaluates. Coloured sky terms were already labelled tints (sky.tint.<label>), so the moon's tint is a patch that joins core's dawn, dusk and overcast (tested). The example mod is docs/modding/examples/two_suns: core's sun off, a red and a white sun, a green moon full every 8 days with its own tint, the renderer's sun path moved to the white sun. It depends on core only. Its Luau tests load it beside weather and pass: core's sun is gone at 08:00, the suns add up at noon, the moon waxes, and a forced storm dims light at the same daylight. rim check is clean. mod_tests runs it all from a temp folder.

## 2026-09-28

Screenshots: the render bench on seed 1 with core, weather and two_suns, dusk view moved to 00:30 by a local change (not committed), at a new moon against a nearly full one. The same ground reads mean RGB (27.2, 37.8, 49.3) and (27.3, 42.0, 49.3): the night goes greener as the moon fills, red and blue unchanged. It's subtle on screen, because the eye adapts to the night. The moon's own colour, path and shadows are 1a17d685.

## 2026-09-28

Clouds under the two suns, the same bench run: whole map clear against the storm, same hour and ground: mean RGB (54.5, 61.5, 50.8) against (52.8, 55.9, 55.3). Dimmer and colder (core's overcast tint), with rain over it. The eye's adaptation softens it on screen; the sim test holds the rule: light at most 0.8 of clear, at the same daylight.

## 2026-09-28

Review fixes:
- The guide's moon sample is a whole patch, so the guide test loads it.
- The moon is full at midnight: a quarter-day offset, since tick 0 is 06:00. It was full at dawn, and the test sampled it waning.
- The storm test forces clear weather for its baseline.
- The example loads warning-free beside weather (tested).
- days and offset outside a cycle are errors, and a cycle is an hour or longer.
- The white sun peaks at 60, so noon stays within daylight's 0 to 100 (tested).
- DESIGN's inputs list names cycle.
- cycle uses u64 maths.
The screenshots above were taken before these changes (white sun 90, moon full at dawn). What they show still holds: the moon greens the night, and the storm dims and cools the suns.
