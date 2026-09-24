---
id: f6e18475-c7b8-407f-b31c-61f7d8769406
title: 'Mod modules: require("@mod/path") limited to declared dependencies'
type: feature
status: done
milestone: plugin-api
assignee: Oddur Sigurdsson
depends_on:
- 3eb7e697-6bb4-4318-90f4-7f4d727b98c7
created: 2026-09-23
updated: 2026-09-24
closed_at: 2026-09-24
priority: p0
api: breaking
effort: m
layer: engine
area: scripting
pillar:
- plugin-first
---

## Why

Today mods share APIs by assigning into the global `rim` table: names collide silently and nothing records who depends on whom. DESIGN.md §10: hard dependencies use `require`.

## What

A script can `return` a table of exports. `require("@core/storyteller")` returns it. Only mods listed in `depends` or `optional` can be required; an optional mod that isn't installed returns nil.

## Acceptance criteria

- [x] `require("@<mod>/<script>")` resolves across mods; `require("./x")` within a mod
- [x] Requiring a mod not in `depends`/`optional` is a load error naming both mods
- [x] Module cycles are a load error
- [x] Storyteller exports `register_incident`; wildlife_plus uses the require form
- [x] Generated `.luaurc` aliases so luau-lsp resolves the same paths

## 2026-09-24

Went with the DESIGN §10 ruling in full: rim is read-only (the add-only proxy from #26 now refuses every write and points at require), and mods share code as modules. Paths are "@mod/scripts/x" (same shape as the UI's "@mod/ui/x", and what the .luaurc aliases resolve) or "./x" relative to the file. Top-level scripts are entries and run at load in name order; subdirectory scripts run only when required; either way once. Exports are frozen when their mod finishes loading, so a mod's own later scripts can extend them but other mods can't patch them. require works only at load. mod.toml gained optional = [...] (orders like load_after; requiring an absent one gives nil); version ranges stay with 0151. Migrated: core/scripts/storyteller.luau exports register_incident, weather/scripts/weather.luau exports weather, and weather's incidents are their own module (incidents.fire) since luau-lsp rightly rejects adding fields to another script's table. API bumped to 0.2. .luaurc is now generated and checked by tests/api_types.rs.
