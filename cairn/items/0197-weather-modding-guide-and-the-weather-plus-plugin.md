---
id: 197
title: Weather modding guide and the weather_plus plugin
type: docs
status: planned
milestone: weather
depends_on:
- 57
- 184
- 187
- 191
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: none
effort: s
layer: plugin
area: docs
pillar:
- plugin-first
---

## Why

The test of the design is that a modder can add weather without us. `wildlife_plus` did this for content and the UI; `weather_plus` does it for weather.

## What

- `docs/modding/weather.md`: the four layers, terms and curves with the input table, regimes, stock and derived fields, pushes and forced weather, visuals, devtools and the climate report.
- `mods/weather_plus`: a `hail` regime (data, particles, harm to plants) and a "first frost" message script; patches one core curve to show patching by label.
- A test loads every Luau and TOML sample in the guide as a mod beside core and fails on any error.

## Acceptance criteria

- [ ] The guide exists and every sample runs in a test
- [ ] `weather_plus` adds a regime and an incident with no engine change
- [ ] A deliberate conflict (two mods replacing one term) is reported
