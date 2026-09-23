---
id: 23
title: 'Typed def schema: terrain, thing, creature, need, designation, start, names'
type: feature
status: done
milestone: foundations
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: additive
effort: m
layer: engine
area: modding
pillar:
- plugin-first
---

## Why

Mods describe content as data. The engine only ever sees merged, validated, typed defs with resolved references.

## Acceptance criteria

- [x] Every def kind deserializes from TOML
- [x] Dangling references fail with the def and file named
- [x] Unknown fields are ignored so plugins can annotate other mods' defs
