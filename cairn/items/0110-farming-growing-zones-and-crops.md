---
id: b1444a26-a018-4b94-9bca-84bb47534094
title: 'Farming: growing zones and crops'
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- eb3b0ac7-9922-4ab2-b9c1-8171e0b7f05f
- 9b569a33-c488-42df-84c2-9bfa83a14b3e
- 6dd4891c-f65e-4707-96f9-e6d46c6cd446
- e1be8ebd-d90f-40a7-bf33-c579ad1f7410
created: 2026-09-22
updated: 2026-09-28
closed_at: 2026-09-27
priority: p0
api: additive
effort: l
layer: engine
area: building
---

## Why

Sustainable food.

## Acceptance criteria

- [x] Growing zones
- [x] Sow and harvest jobs
- [x] Fertility from terrain

## 2026-09-23

Weather sprint (0179) provides the substrate: plants grow from field terms (0191: temperature, wetness, light, fertility), ground wetness (0187), terrain properties including fertility (0185, which replaces the 'Fertility from terrain' criterion here), and a calendar with seasons (0182). Farming adds sowing, crop defs with growing seasons, tilled soil (a terrain prop change), irrigation as wetness emitters, and harvest timing. Deep-farming ideas to plan here: soil fertility depletion and fallow years, crop rotation, frost dates, greenhouses as heated rooms, mulch and drainage.

## 2026-09-23

Weather sprint cut to the lean version: the per-cell items (0185 terrain properties, 0186 stock fields, 0187 wetness and snow, 0191 plant growth, 0192 movement) now sit in Crafting beside this item. Farming depends on `mods/weather` for seasons and rain; wetness is the weather plugin's first stock field. Humidity was cut from v1 (no reader); add it with wetness if drying needs it.

## 2026-09-27

Farming is a plugin, mods/farming, depending on weather, since DESIGN §5 puts depth in plugins. The engine provides three mechanisms, and nothing in them names a crop:
- A growing zone: Zone.plant, and a filter that takes nothing, so every store path ignores it and items dropped there are hauled away. The commands are GrowZone and ZonePlant. Its crop is remapped by id on load; one whose crop is gone is cleared, with a note.
- build.by: a buildable raised by another work type. Blueprints are searched per raising work type, work_waiting counts them there, the construct job uses that work's skill, and the build menu leaves them out.
- systems::tend, every plant pass:
  - lays the crop's plan on each empty cell, with no fixture, item or floor, while the crop's rate there is above 0;
  - takes back a plan nobody has started once the rate isn't;
  - marks a grown crop in its zone with its first destroying harvest.
A built plant starts as a seedling (from the plants item).
Content: farming:sow (plants skill, auto 6/3); potatoes (food 0.1, 7 days, frost-tender below -1°); flax (fibre-tagged, 9 days, hardier). Both read temperature, light, fertility and weather:wetness.
Client: a Grow tool per sowable crop in the Zones tray, and fields drawn green. The Stockpiles sheet, the store tabs and the store menu entries skip growing zones. farming/ui adds a Field inspector tab and zone menu rows to change the crop. The UI API changes are additive: view.crops, act.zone_plant, and Zone.plant/plant_label.
Criterion 3 (fertility from terrain) is met through 9b569a33's props. tests/farming.rs shows rich soil growing faster than dirt.
Not done:
- no seeds (sowing is free);
- no crop rotation or depletion;
- no sowing by date or season window beyond the rate;
- growing-zone drag preview left for green-forest's ZonePreview (11080b20) to pick up.

## 2026-09-28

Rebased onto green-forest's zones work (ZonePreview, the violet theme). A growing zone draws green with the same wash and edges: zone_edge picks GROW for a field. A Grow drag's preview joins only a field of the same crop, and a new field previews green. The Work sheet goes from 760 to 816 px, one 56 px column wider, because the Sow column pushed Haul, the last column, out of the window, and rim_ui's board drag tests pressed outside the grid. It doesn't scroll sideways, so each new work type costs a column's width.
