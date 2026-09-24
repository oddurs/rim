---
id: c7f07804-10de-4aa5-958b-1a3fcd1d353d
title: Weather modding guide
type: docs
status: done
milestone: weather
depends_on:
- cee5648a-9714-4498-9c57-1f46253100d0
- 7c50b502-5e27-4807-a36c-0654fe9aec97
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
priority: p1
api: none
effort: s
layer: tooling
area: docs
pillar:
- plugin-first
---

## Why

The test of the design is that a modder can add weather without us.

## What

- `docs/modding/weather.md`: terms and curves, the calendar, pushes, script data and events, and the weather plugin's API (register a type, force weather, read the forecast), and how channels drive visuals.
- A test loads every TOML and Luau sample in the guide as a mod beside core and weather, and fails on any error.

## Acceptance criteria

- [x] The guide exists and every sample runs in a test
- [x] A sample adds a weather type with no engine change

## 2026-09-23

docs/modding/weather.md: who owns what, terms and curves, patching a term, named contributions, calendar, script data and events, the weather plugin's API, the sky, what the renderer reads, tools. tests/weather_guide.rs loads every toml/lua block (except ones marked <!-- not a sample -->, which are API listings) as a mod beside core and weather and runs half a day: no load or script errors, the drizzle type registers, the two suns and green moon take effect. No weather_plus plugin: the lean sprint made mods/weather itself the proof that weather needs no engine change.
