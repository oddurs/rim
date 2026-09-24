---
id: 7c50b502-5e27-4807-a36c-0654fe9aec97
title: 'The weather plugin: seasons, weather types and a forecast'
type: feature
status: done
milestone: weather
depends_on:
- eb3b0ac7-9922-4ab2-b9c1-8171e0b7f05f
- 3bb54ba3-21f9-42a4-9f7c-8299b5db1db5
- b023a009-64d7-4113-81eb-4220b069b74b
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
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

- [x] The forecast is what happens unless forced (test over a year, several seeds)
- [x] Type frequencies over many years match their weights within 5%
- [x] Channels ease between types with no jumps
- [x] Removing the plugin leaves the core game unchanged
- [x] Two runs of a year hash identically

## 2026-09-23

mods/weather: seasons by patching core's temperature terms (mean over the year, daily swing narrower in winter, damped to 80% under cloud); five types (clear, overcast, rain/snow, storm/blizzard, fog) registered in Luau; a queue of four picked with the world RNG; channels pushed as 'weather' with each type's blend hours. Forecast truth: 3 seeds x a full year, every change is the forecast moving up, except incident-forced ones (checked separately, under a fifth of changes). Frequencies: 20,000 picks match weight shares within 5% (tested via rim.weather.pick, the same function the queue uses, rather than 40 simulated years). Channels never move more than 5% of their range per 20 ticks. Core alone: no pushes, no script data, cloud/rain/wind/fog 0. A year hashes identically twice. The hook caches when the weather ends: 0.2 µs a call. Weather types move to data with 0208. Tests: crates/rim_sim/tests/climate.rs, weather.rs, weather_guide.rs (release, as CI runs them).
