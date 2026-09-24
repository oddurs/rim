---
id: 182
title: Calendar, seasons and script events
type: feature
status: done
milestone: weather
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
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

- [x] The calendar loads from core and can be patched
- [x] `season_changed` fires once per season at the right tick (test over two years)
- [x] A script event reaches handlers in another mod
- [x] The top bar shows the season and day

## 2026-09-23

mods/core/defs/calendar.toml: 60-day year, four seasons, start day 8 (0-based: Spring 9). One [[calendar]] allowed; patch calendar/core. World::year/season/day_of_season/clock; season_changed fires at the day boundary where the season index changes (test jumps across 8 boundaries over two years, each seen once). rim.emit(name, table) queues a GameEvent::Script dispatched to rim.on handlers in any mod (test: mod a emits, mod b hears). Top bar: 'Day 1 · Spring 9'. Tests: crates/rim_sim/tests/climate.rs, weather.rs, weather_guide.rs (release, as CI runs them).
