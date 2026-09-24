---
id: 110
title: 'Farming: growing zones and crops'
type: feature
status: backlog
milestone: crafting
depends_on:
- 182
- 185
- 187
- 191
created: 2026-09-22
updated: 2026-09-23
priority: p0
api: additive
effort: l
layer: engine
area: building
---

## Why

Sustainable food.

## Acceptance criteria

- [ ] Growing zones
- [ ] Sow and harvest jobs
- [ ] Fertility from terrain

## 2026-09-23

Weather sprint (0179) provides the substrate: plants grow from field terms (0191: temperature, wetness, light, fertility), ground wetness (0187), terrain properties including fertility (0185, which replaces the 'Fertility from terrain' criterion here), and a calendar with seasons (0182). Farming adds sowing, crop defs with growing seasons, tilled soil (a terrain prop change), irrigation as wetness emitters, and harvest timing. Deep-farming ideas to plan here: soil fertility depletion and fallow years, crop rotation, frost dates, greenhouses as heated rooms, mulch and drainage.

## 2026-09-23

Weather sprint cut to the lean version: the per-cell items (0185 terrain properties, 0186 stock fields, 0187 wetness and snow, 0191 plant growth, 0192 movement) now sit in Crafting beside this item. Farming depends on `mods/weather` for seasons and rain; wetness is the weather plugin's first stock field. Humidity was cut from v1 (no reader); add it with wetness if drying needs it.
