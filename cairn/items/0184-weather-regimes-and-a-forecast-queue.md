---
id: 184
title: Weather regimes and a forecast queue
type: feature
status: planned
milestone: weather
depends_on:
- 183
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: additive
effort: m
layer: engine
area: sim
pillar:
- plugin-first
- determinism
- survival
---

## Why

Weather should be a thing that happens, with a name, a duration and a forecast, not a noisy number. Regimes give the storyteller, the UI and the renderer one shared answer to "what is the weather?", and a queue picked in advance makes the forecast true.

## What

- `[[weather]]` defs: `label`, `duration_hours = [min, max]`, `blend_hours`, `weight = [terms]`, `after = { id = factor }`, `set = { key = value | [min, max] }`, `visuals = {...}` (client only).
- Engine state: the current regime, the next three, and a blend factor. A regime is picked with the world RNG when it joins the queue.
- `weather = key` inputs read the blended setting; a key a regime doesn't set falls back to `[weather_defaults]`, and a key nobody sets is a load error.
- `rim.weather()`, `rim.forecast()`, `rim.force_weather(id, hours)` (replaces the head of the queue and re-picks after it), event `weather_changed { from, to }`.
- Core regimes: clear, cloudy, rain, storm, fog, snow, weighted by season and temperature. Snow only below freezing, fog in still humid mornings.
- Saved in the state hash.

## Acceptance criteria

- [ ] The forecast shown at any tick is what happens, unless forced (test over a year, 10 seeds)
- [ ] Regime frequencies over 40 simulated years match their weights within 5%
- [ ] Transitions blend over `blend_hours` with no step in any channel
- [ ] `force_weather` works from a script and fires `weather_changed`
- [ ] Two runs of a year hash identically
