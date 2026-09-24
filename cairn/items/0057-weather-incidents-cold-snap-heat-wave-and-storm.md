---
id: 57
title: 'Weather incidents: cold snap, heat wave and storm'
type: content
status: planned
milestone: weather
depends_on:
- 184
created: 2026-09-22
updated: 2026-09-23
priority: p1
api: additive
effort: s
layer: plugin
area: storyteller
pillar:
- survival
- plugin-first
---

## Why

Exposure needs pressure from weather that arrives as an event with a name: a cold snap to ride out, a storm that sends everyone inside.

## What

In `mods/weather`, registered with core's storyteller:

- `cold_snap`: `rim.push_ambient("temperature", "cold_snap", -12, 48, 3)` with a warning message; not in summer.
- `heat_wave`: +10°C for two days in summer.
- `storm`: `rim.weather.force("storm", 12)`.

## Acceptance criteria

- [ ] Weather state readable by the engine (the regimes item provides it)
- [ ] Cold snap, heat wave and storm scripted in `mods/weather` with pushes and forced weather
- [ ] Each shows in the breakdown and the forecast
- [ ] Balance: a cold snap in the first week is survivable with a hut (harness)
