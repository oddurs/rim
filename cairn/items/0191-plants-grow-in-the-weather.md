---
id: e1be8ebd-d90f-40a7-bf33-c579ad1f7410
title: Plants grow in the weather
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- 3bb54ba3-21f9-42a4-9f7c-8299b5db1db5
- 9b569a33-c488-42df-84c2-9bfa83a14b3e
- 6dd4891c-f65e-4707-96f9-e6d46c6cd446
created: 2026-09-23
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] Growth rates follow temperature, wetness, light and fertility as declared (tests with fixed conditions)
- [x] Berry bushes don't regrow in winter; a hard frost kills tender plants
- [x] Wild plants spread more on wet fertile ground than dry sand over a year
- [x] Plant pass cost with 5,000 plants measured and recorded here
- [x] `regrow_days` still loads for mods that use it

## 2026-09-23

Moved to Crafting: plant growth pays off with farming. Rain and seasons from `mods/weather` are its inputs.

## 2026-09-27

Built:
- grow = { days, rate, harm, heal, stages, after_harvest } on thing defs. Terms are read at the plant's cell through Fields::eval_at (field, terrain, near, sky).
- A Growth component { progress, health, dormant } in 1/10000ths.
- systems::grow in the 250-tick plant pass works out a quarter of the plants each pass, by entity id % 4, so each plant every 1000 ticks. Changes apply in id order, and deaths despawn.
Harvests a plant survives regrow with it: the harvest waits (Regrow ready_at = WHEN_GROWN) until the pass sees the plant grown again, and a harvest cuts it back to after_harvest. A harvest that gives regrow_days keeps to them (criterion 5), so every mod on days is unchanged. That covers the orchard tests on the oak and primitive's clay and bog iron banks. A felled plant yields by how grown it is.
Spread weights each cell's chance by the plant's rate there, clamped to 0..2. It draws the same RNG as before, and a spread plant starts as a seedling. So does a plant built from a blueprint, the hook farming's sowing can use.
Core: oak (16 days) and berry bush (1.5 days, after_harvest 0.4) read temperature, light and fertility. The weather plugin patches in weather:wetness (mods/weather/defs/plants.toml; ids qualified, since a bare one resolves in the patched mod). wildlife_plus's balance patch now sets the bush's grow days (1.0) instead of regrow_days.
Client: a plant is drawn at 50–100% size by stage, faded toward grey while dormant and browned below half health. The hover line says '40% grown', 'dormant' or 'withering'.
Saves: an optional engine:growth section, no format bump. A plant from an older save loads grown. Growth is in the state hash.
Measured (tests/growth.rs, release, M1 laptop): the plant pass with 6120 plants (5000 sown plus the map's own) costs 0.20 ms a pass, 0.0008 ms a tick. A year of spread on the weather plugin's ground: rich soil 0.035 new plants a cell, sand 0.007.
