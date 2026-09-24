---
id: b0da41ef-9eda-479e-b080-0d0b9dc0129d
title: 'Custom def kinds: plugins declare their own data'
type: feature
status: done
milestone: plugin-api
assignee: Oddur Sigurdsson
created: 2026-09-23
updated: 2026-09-24
closed_at: 2026-09-24
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

- [x] A plugin-declared kind loads from several mods, with patches and conflicts
- [x] Scripts read the entries
- [x] `mods/weather` moves its weather types from Luau registration to data

## 2026-09-24

Built in 0143's form so neither is redone: [[kind]] in a mod's defs with an optional [kind.fields] schema (string/int/float/bool/table/list/any, with defaults), not a kinds list in mod.toml. Other mods write [[weather.type]] (TOML's dotted header, since a colon can't be a bare key); the declaring mod may write [[type]]. Entries share the built-in path: one index, so patches (target = "weather:type/weather:storm") and conflict warnings just work. Stored in DefDb::mod_defs as plain data; rim.defs(kind) returns fresh copies (resolving a bare kind against the calling mod) so a mod can adjust what it registers. Weather's five types moved to defs/types.toml; the weight closures became data (weight x season x follows, first_day), and a year of weather on seed 3 is identical day by day to before. Type ids are now qualified (weather:storm); weather's functions still accept its own bare names.
