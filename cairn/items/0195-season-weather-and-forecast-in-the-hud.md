---
id: 195
title: Season, weather and forecast in the HUD
type: feature
status: done
milestone: weather
depends_on:
- 181
- 184
- 207
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
priority: p1
api: additive
effort: s
layer: client
area: ui
pillar:
- plugin-first
---

## Why

A forecast only helps if you can see it, and values should explain themselves to players as well as modders.

## What

- Core's top bar shows the date and season (core owns the calendar).
- `mods/weather/ui/`: a weather readout (type, temperature, wind) extending `core:topbar.right`; clicking opens a forecast panel with the next three types and when they start, plus the temperature breakdown.
- Toasts when a storm or snow begins and when the season changes.

## Acceptance criteria

- [x] The readout and panel live in `mods/weather/ui/`, extendable by id
- [x] The breakdown lists each term and push
- [x] The autotest opens the forecast panel by node id and checks it against the forecast data

## 2026-09-23

mods/weather/ui/weather.luau extends core:topbar.right with weather:readout ('Rain · 7°C · ↘ 5 m/s'); clicking toggles weather:forecast.panel (right column) with the next three types and when they start, and the temperature breakdown (view.explain). Weather colours are mod theme tokens (weather_cold, weather_warm). Core's top bar shows the date. Messages cover storms, snow and season changes. Autotest: opens the panel by node id, checks it has a row per queued entry and the breakdown. New view API: date, ambient, explain, data, ticks_per_day (documented in docs/modding/ui.md).
