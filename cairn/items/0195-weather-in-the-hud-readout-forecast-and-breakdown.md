---
id: 195
title: 'Weather in the HUD: readout, forecast and breakdown'
type: feature
status: planned
milestone: weather
depends_on:
- 181
- 184
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: additive
effort: s
layer: client
area: ui
pillar:
- plugin-first
---

## Why

A forecast is only useful if you can see it, and a hackable system should explain itself to players as well as modders.

## What

- UI API: `view.weather()`, `view.forecast()`, `view.season()`, `view.explain(field, x, y)` (per-term breakdown).
- `core:weather` in the top bar: regime, feels-like temperature and wind. Clicking opens a panel with the next three regimes, the season, and today's growing conditions.
- The hover readout adds wetness, snow and feels-like; hovering a value shows its breakdown as a tooltip.
- Toasts on `weather_changed` to storm or snow and on `season_changed`.

## Acceptance criteria

- [ ] The top bar and forecast panel are in `mods/core/ui/`, extendable by id
- [ ] The breakdown tooltip lists each term with its contribution
- [ ] The autotest opens the forecast panel by node id and checks it matches `rim.forecast()`
