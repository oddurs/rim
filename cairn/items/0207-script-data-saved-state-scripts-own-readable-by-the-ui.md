---
id: b023a009-64d7-4113-81eb-4220b069b74b
title: 'Script data: saved state scripts own, readable by the UI'
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
area: scripting
pillar:
- plugin-first
- determinism
---

## Why

A plugin's state lives in Luau locals today: the storyteller's timers, and now the weather plugin's forecast. That state isn't in the state hash, won't be saved (0060), and the UI VM can't read it, so a forecast panel has nothing to show. Plugins need somewhere to keep state that belongs to the world.

## What

- `rim.set_data(key, value)` / `rim.get_data(key)`: plain data (nil, booleans, numbers, strings, tables of those) stored in the world under namespaced keys (`weather:forecast`).
- Included in the state hash; serialised with the world when saves arrive (0060).
- UI: `view.data(key)` returns a read-only copy.
- Functions, userdata and cycles are rejected with an error naming the key.

## Acceptance criteria

- [x] Scripts can store and read back nested tables
- [x] Stored data changes the state hash
- [x] The UI reads it through `view.data`
- [x] Non-data values are rejected with a clear error

## 2026-09-23

crates/rim_sim/src/data.rs: Data (bool, int, num, string, sorted table) with a stable hash; rim.set_data/get_data, view.data in the UI VM. Functions are refused with 'only plain data can be stored, not a function'. World.data is in state_hash. The weather forecast lives here as weather:forecast. Tests: crates/rim_sim/tests/climate.rs, weather.rs, weather_guide.rs (release, as CI runs them).
