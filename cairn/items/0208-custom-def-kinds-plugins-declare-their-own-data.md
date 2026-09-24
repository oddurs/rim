---
id: 208
title: 'Custom def kinds: plugins declare their own data'
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: additive
effort: m
layer: engine
area: modding
pillar:
- plugin-first
---

## Why

The loader ignores def kinds it doesn't know, so a plugin can't offer TOML data for other mods to extend: weather types, mood thoughts, incidents and crops all end up as Luau registration calls instead of patchable, conflict-checked data. Found while planning the Weather sprint (DESIGN.md §4c).

## What

- A mod declares a kind in `mod.toml` (`kinds = ["weather"]`); entries of that kind from any mod that depends on it load, patch and report conflicts like built-in defs.
- Scripts read them as tables: `rim.defs("weather")`.
- Optional schema (required keys and types) checked at load.

## Acceptance criteria

- [ ] A plugin-declared kind loads from several mods, with patches and conflicts
- [ ] Scripts read the entries
- [ ] `mods/weather` moves its weather types from Luau registration to data
