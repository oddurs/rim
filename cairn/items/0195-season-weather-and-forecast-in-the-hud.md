---
id: 195
title: Season, weather and forecast in the HUD
type: feature
status: planned
milestone: weather
depends_on:
- 181
- 184
- 207
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

A forecast only helps if you can see it, and values should explain themselves to players as well as modders.

## What

- Core's top bar shows the date and season (core owns the calendar).
- `mods/weather/ui/`: a weather readout (type, temperature, wind) extending `core:topbar.right`; clicking opens a forecast panel with the next three types and when they start, plus the temperature breakdown.
- Toasts when a storm or snow begins and when the season changes.

## Acceptance criteria

- [ ] The readout and panel live in `mods/weather/ui/`, extendable by id
- [ ] The breakdown lists each term and push
- [ ] The autotest opens the forecast panel by node id and checks it against the forecast data
