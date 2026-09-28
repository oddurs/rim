---
id: a9e6b195-bf61-45b6-ab66-f90ee784166c
title: Core's sun and moon light the sim from where they are
type: content
status: backlog
milestone: lighting
depends_on:
- 0a27bfbb-276b-4b28-a7cc-1e02a3f8dc91
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
effort: m
layer: core
area: sim
---

## Why

The user decided on 2026-09-28 that the moon, and the sun with it, light the simulation, not only the picture. Once 0a27bfbb puts the bodies in the sim, core's light should come from where they are.

## What

- The `sun` term of daylight reads the sun body's `up` and `altitude` in place of the fixed hour curve. That gives a twilight ramp, a brighter high sun, and shorter, dimmer midwinter days.
- The moon gets a daylight term: `phase` × `up` × a strength of about 1.5 at full. Cloud dims it as it dims the sun, through `light`. So the sim's night light is above 0 under a full moon, and 0 at new moon.
- Core's moon drops its render-only `scale`/`of`, while the two-suns example's green moon keeps its own choice. DESIGN.md and the lighting milestone text now say the moon lights the sim (the user's decision, 2026-09-28).
- Balance: report the stone-age sweep, the crosscheck (seed 4 lives 60 days), soak and climate, before and after. Plants grow a little under a full moon; nothing else should change on purpose.

## Acceptance criteria

- [ ] Under a clear full moon the sim's night light is above 0, and at new moon it's 0 (test)
- [ ] Daylight hours in the light field vary by season across a year (test)
- [ ] A plant grows on full-moon nights and not on new-moon nights (test)
- [ ] The PR reports the stone-age sweep, crosscheck, soak and climate before and after, with no regression it doesn't explain
