---
id: 65f0b723-5310-4dc6-a0f6-ba1223edb25b
title: 'Script data belongs to its mod: set_data namespaced by the engine'
type: feature
status: backlog
milestone: persistence
created: 2026-09-22
updated: 2026-09-24
priority: p0
api: additive
effort: m
layer: engine
area: scripting
pillar:
- plugin-first
---

## Why

A mod's saved state is its script data (DESIGN.md §7a), and a migration or a
parked section must be exactly one mod's. Today the `your_mod:key` namespace
is only a convention, so one mod can write another's data.

## What

The engine qualifies keys with the calling mod: a bare key is the caller's,
a qualified key is allowed only in the caller's own namespace. Reading
another mod's data stays allowed (the UI reads `weather:forecast`). No
separate `rim.state` table: script data already is it.

## Acceptance criteria

- [ ] `rim.set_data("x", v)` from mod `m` writes `m:x`
- [ ] Writing into another mod's namespace is an error naming both mods
- [ ] Script data can be read back grouped by mod, one group per section
- [ ] Docs (api-scripts.md, scripting.md) say so

## 2026-09-24

From the save-format decision (DESIGN.md §7a): the Luau VM isn't saved, so any state a mod must keep lives in script data (rim.set_data: saved and hashed). Today core's storyteller keeps last/last_threat in Luau locals, which a load would reset and the desync check can't see. Move it to script data, and consider whether a per-mod rim.state table is still needed on top of set_data's namespaced keys.

Core's storyteller now keeps last/last_threat in script data (core:storyteller) instead of Luau locals, so the memory is hashed and will be saved; tested in mods/core/tests/storyteller.luau. What remains here: whether mods need a rim.state table beyond set_data's namespaced keys, and the save/load itself (0061).
