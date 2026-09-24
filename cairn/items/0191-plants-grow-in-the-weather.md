---
id: 191
title: Plants grow in the weather
type: feature
status: planned
milestone: weather
depends_on:
- 183
- 185
- 187
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: additive
effort: m
layer: engine
area: sim
pillar:
- survival
- growth
- plugin-first
---

## Why

Weather has to matter to food. Growth driven by fields makes spring a flush, summer drought a worry and winter a famine you prepare for. It is also the base farming (0110) sows onto.

## What

- `grow = { days, rate = [terms], harm = [terms], stages }` on plant defs. A `Growth` component holds stage 0–1 and health.
- Updated in the existing 250-tick plant pass, staggered by entity. Rate multiplies base speed; harm damages and kills (frost on tender plants, drought, flooding).
- Berry bushes: regrow through growth instead of the fixed `regrow_days`; no berries in winter. Trees: saplings from spreading grow to full size before they can be chopped for full yield.
- Wild spread picks cells weighted by the plant's growth rate there.
- The client draws growth stage (size) and dormant or damaged plants (colour).

## Acceptance criteria

- [ ] Growth rates follow temperature, wetness, light and fertility as declared (tests with fixed conditions)
- [ ] Berry bushes don't regrow in winter; a hard frost kills tender plants
- [ ] Wild plants spread more on wet fertile ground than dry sand over a year
- [ ] Plant pass cost with 5,000 plants measured and recorded here
- [ ] `regrow_days` still loads for mods that use it
