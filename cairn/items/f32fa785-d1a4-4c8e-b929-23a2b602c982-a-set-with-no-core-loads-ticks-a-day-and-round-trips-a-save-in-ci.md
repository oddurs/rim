---
id: f32fa785-d1a4-4c8e-b929-23a2b602c982
title: A set with no core loads, ticks a day and round-trips a save, in CI
type: feature
status: backlog
milestone: proving-ground
created: 2026-09-27
updated: 2026-09-27
priority: p3
api: none
effort: s
layer: engine
area: tests
pillar:
- plugin-first
---

## Why

DESIGN.md §5 says core is a plugin with no special case in the engine, so a
total conversion replaces it. Nothing tests that. The day some engine code
assumes a `core:` id, total conversions silently stop being possible.

Factorio defaults a mod's dependencies to `["base"]` and lets `[]` opt out.
Paradox needs `replace_path` because its base game isn't a mod. rim avoids
that by construction, but only a test keeps it true.

## What

- A fixture under `crates/rim_sim/tests/fixtures/no_core/` with two mods:
  - `vocab`, which declares the minimum the engine needs: a terrain, a
    creature, a need, a calendar, a sky, a priority scale, a store
    priority and a start;
  - `content`, which adds one thing and one script.
- A test loads the fixture directory, runs one in-game day, saves, loads and
  compares.
- docs/modding gains a short page, "Replacing core": what a replacement
  must declare, taken from the fixture.

## Acceptance criteria

- [ ] The fixture loads with no mod named `core`, runs a day, and round-trips a save (test in the default suite)
- [ ] The test fails with a clear message if engine code requires a `core:` id
- [ ] docs/modding/replacing-core.md lists what the fixture had to declare
