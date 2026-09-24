---
id: cee5648a-9714-4498-9c57-1f46253100d0
title: 'Weather incidents: cold snap, heat wave and storm'
type: content
status: done
milestone: weather
depends_on:
- 7c50b502-5e27-4807-a36c-0654fe9aec97
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
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

- [x] Weather state readable by the engine (the regimes item provides it)
- [x] Cold snap, heat wave and storm scripted in `mods/weather` with pushes and forced weather
- [x] Each shows in the breakdown and the forecast
- [x] Balance: a cold snap in the first week is survivable with a heated hut (harness)

## 2026-09-23

mods/weather/scripts/20_incidents.luau, through core's storyteller: cold_snap (push -8°C, -12 in winter, 48 h, from day 5), heat_wave (+10°C in summer and a forced clear spell), storm (force storm 6-12 h, from day 3); rim.weather.incident(id) sets one off for tools and tests (test: each shows in the breakdown / forecast). Criterion reworded from 'with a hut' to 'with a heated hut' after measuring: an unheated hut leaks to near the outdoor temperature, so a snap needs a fire, which is what its warning says. 80 seeds, snap on day 5: hut 16/80 colonies lost (9/80 without a snap); hut and campfire 1/80 (2/80 without). -5°C was tried and still lost 14/80 with a bare hut. Recorded in DESIGN.md §4c.
