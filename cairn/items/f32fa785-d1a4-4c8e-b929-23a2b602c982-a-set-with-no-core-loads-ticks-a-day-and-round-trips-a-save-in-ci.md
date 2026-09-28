---
id: f32fa785-d1a4-4c8e-b929-23a2b602c982
title: A set with no core loads, ticks a day and round-trips a save, in CI
type: feature
status: done
milestone: proving-ground
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
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

- [x] The fixture loads with no mod named `core`, runs a day, and round-trips a save (test in the default suite)
- [x] The test fails with a clear message if engine code requires a `core:` id
- [x] docs/modding/replacing-core.md lists what the fixture had to declare

## 2026-09-28

The engine already runs a game with no core. The least that loads is a [[terrain]] (with a gen band), a [[creature]] and a [[start]]; the item guessed need, calendar, sky, priority scale and store priority too, and none is required ([[names]] isn't either: a colonist takes its creature's label). tests/no_core.rs loads the fixture (vocab plus a content mod with a thing and a script that counts hours in saved data and lays a pebble each), runs a day (25 hours, 25 pebbles), and round-trips a save to the same state hash. It wraps the run so a failure says 'Engine code may need a core: id, which DESIGN.md §5 rules out': shown by planting defs.thing_id("core:wood").expect(..) in Sim::build ('a game with no core mod panicked: core:wood. Engine code may need...'), reverted. The loader's 'is the core mod installed?' errors now name what's missing and point at the guide. docs/modding/replacing-core.md lists the three, and a second test keeps its table in step with the fixture.
