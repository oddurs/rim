---
id: 182
title: 'Calendar: year, seasons and day of year'
type: feature
status: planned
milestone: weather
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: additive
effort: s
layer: engine
area: sim
pillar:
- plugin-first
- growth
---

## Why

Seasons are curves over the year, so the sim needs to know what time of year it is. Farming, storytelling and incidents all key off it.

## What

- `[climate]` def: `year_days` (core: 60), `seasons = ["spring", "summer", "autumn", "winter"]`, `start_day` (core: day 8 of spring, late spring).
- `World::year_fraction()`, `season()`, `day_of_year()`; `GameEvent::SeasonChanged`.
- Scripts: `rim.year()`, `rim.season()`, `rim.on("season_changed", ...)`. UI: `view.season()`, `view.date()`.
- The top bar shows the season and day ("Late spring, day 8").

## Acceptance criteria

- [ ] `[climate]` loads from core, and a mod can patch `year_days` and season names
- [ ] Season changes fire one event at the right tick (test over two years)
- [ ] Scripts and the UI can read the date and season
- [ ] The top bar shows the season
