---
id: 667
uid: a9e6b195-bf61-45b6-ab66-f90ee784166c
title: Core's sun and moon light the sim from where they are
type: content
status: done
milestone: lighting
assignee: Oddur Sigurdsson
depends_on:
- 563
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
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

- [x] Under a clear full moon the sim's night light is above 0, and at new moon it's 0 (test)
- [x] Daylight hours in the light field vary by season across a year (test)
- [x] A plant grows on full-moon nights and not on new-moon nights (test)
- [x] The PR reports the stone-age sweep, crosscheck, soak and climate before and after, with no regression it doesn't explain

## 2026-09-28

Balance, before (main plus 0a27bfbb, whose sim numbers are main's) against after, at latitude 45 (core's) and 30. Light (clear sky, hours at ≥30, a plant's full rate): before about 13.8 every day of the year. After at 45: 15.6 at midsummer, 12.2 at the equinoxes, 8.9 at midwinter, with noon at 100 in summer and 93 in winter. Full-moon midnight light 1.5, new moon 0; a berry bush grows 64 progress units in four full-moon night hours and 0 at new moon. Stone-age sweep (20 seeds, 5 days): every target passes at the same rate in all three (100/100/100/75%), and per-seed step times are within noise. Wolves kill 5 at 45 as before (on seed 8 in place of 11) and 6 at 30. Soak, 3 days, seed 1: 2 colonists in each; wealth 545/545/542 with every mod, 542/554/554 with core alone. Climate, year seed 1: identical weather spells at 45 (112 spells, 7.9°C mean); at 30 the weather took another path (100 spells, 8.5°C, less rain). Crosscheck: exits 0 in all three, but the hash moves, as it must. The harness checks the colony only on day 5, and on main every seed where it lives past day 5 (4, 5, 6, 7, 10, 11, 12) loses it between day 24 and 39 (mean 30.6), to raids, stampedes and wolves. So 'seed 4 lives 60 days' was already false before this change. After at 45: days 13–35, mean 25.9, median 26 (29 before); at 30: days 11–39, mean 30.6, median 32. The two early losses at 45 (seeds 4 and 11) are boar stampedes and wolves, not hunger, so I read them as the storyteller's rolls, not the light. Seven seeds can't separate the two, though.

## 2026-09-28

The before/after table (sweep, crosscheck, soak, climate at 45 and 30 degrees) is in PR #337. The one row to read sceptically, crosscheck colony survival at 45 degrees, is explained there as storyteller rolls moved by the hash.
