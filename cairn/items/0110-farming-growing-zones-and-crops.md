---
id: b1444a26-a018-4b94-9bca-84bb47534094
title: 'Farming: growing zones and crops'
type: feature
status: backlog
milestone: crafting
depends_on:
- eb3b0ac7-9922-4ab2-b9c1-8171e0b7f05f
- 9b569a33-c488-42df-84c2-9bfa83a14b3e
- 6dd4891c-f65e-4707-96f9-e6d46c6cd446
- e1be8ebd-d90f-40a7-bf33-c579ad1f7410
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
