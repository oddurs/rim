---
id: 182
title: Calendar, seasons and script events
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

Seasons are curves over the year, so the sim needs a calendar, and plugins need to tell each other things ("the weather changed") the way the engine tells them ("a pawn died").

## What

- `[[calendar]]` in core: `year_days = 60`, `seasons = ["spring", "summer", "autumn", "winter"]`, `start_day = 9`. The calendar is shared vocabulary, so core owns it; seasons *mattering* is the weather plugin's job.
- `World::year()`, `season()`, `day_of_year()`; `season_changed` event.
- `rim.year()`, `rim.season()`, `rim.date()`; `rim.emit(name, table)` queues a script event for `rim.on` handlers.
- UI: `view.date()`; core's top bar shows "Spring 9".

## Acceptance criteria

- [ ] The calendar loads from core and can be patched
- [ ] `season_changed` fires once per season at the right tick (test over two years)
- [ ] A script event reaches handlers in another mod
- [ ] The top bar shows the season and day
