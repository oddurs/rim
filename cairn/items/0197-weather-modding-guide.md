---
id: 197
title: Weather modding guide
type: docs
status: planned
milestone: weather
depends_on:
- 57
- 184
created: 2026-09-23
updated: 2026-09-23
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

- [ ] The guide exists and every sample runs in a test
- [ ] A sample adds a weather type with no engine change
