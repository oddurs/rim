---
id: 184
title: 'The weather plugin: seasons, weather types and a forecast'
type: feature
status: planned
milestone: weather
depends_on:
- 182
- 183
- 207
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: none
effort: m
layer: plugin
area: scripting
pillar:
- plugin-first
- determinism
- survival
---

## Why

Weather is depth, not skeleton, so it is a first-party plugin built only on the public API. If it can't be built that way, the API is wrong (DESIGN.md §4c).

## What

- `mods/weather` (depends on core, enabled by default).
- **Seasons:** patches core's `temperature` terms by label: the mean follows a year curve (spring starts like today; winter around −6°C), and cloud damps the daily swing. Light dims under cloud through core's `light` term: the plugin sets `cloud` and never patches `daylight`, so it works under any sky.
- **Weather types**, registered in Luau (`rim.weather.register{...}`; data once custom def kinds land): clear, cloudy, rain, storm, fog. Each has a duration range, blend hours, a weight function of season and the previous type, and channel settings (cloud, precipitation, wind, fog, temperature offset). Precipitation below 0°C is snow.
- **Forecast:** the current type and the next three, each picked with the world RNG when it joins the queue, kept in script data (`weather:forecast`). Changes push the channels with easing.
- `rim.weather.current()`, `forecast()`, `force(id, hours)`; `weather_changed` via `rim.emit`.

## Acceptance criteria

- [ ] The forecast is what happens unless forced (test over a year, several seeds)
- [ ] Type frequencies over many years match their weights within 5%
- [ ] Channels ease between types with no jumps
- [ ] Removing the plugin leaves the core game unchanged
- [ ] Two runs of a year hash identically
