---
id: 65f0b723-5310-4dc6-a0f6-ba1223edb25b
title: 'Script data belongs to its mod: set_data namespaced by the engine'
type: feature
status: done
milestone: persistence
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-24
closed_at: 2026-09-24
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

- [x] `rim.set_data("x", v)` from mod `m` writes `m:x`
- [x] Writing into another mod's namespace is an error naming both mods
- [x] Every stored key is qualified by its mod, so a mod's data is one key range
- [x] Docs (api-scripts.md, scripting.md) say so

## 2026-09-24

From the save-format decision (DESIGN.md §7a): the Luau VM isn't saved, so any state a mod must keep lives in script data (rim.set_data: saved and hashed). Today core's storyteller keeps last/last_threat in Luau locals, which a load would reset and the desync check can't see. Move it to script data, and consider whether a per-mod rim.state table is still needed on top of set_data's namespaced keys.

Core's storyteller now keeps last/last_threat in script data (core:storyteller) instead of Luau locals, so the memory is hashed and will be saved; tested in mods/core/tests/storyteller.luau. What remains here: whether mods need a rim.state table beyond set_data's namespaced keys, and the save/load itself (0061).

## 2026-09-24

set_data and get_data qualify a bare key with the calling mod (calling_mod, falling back to the loading mod like emit). A write under another mod's prefix is an error naming both; reads of another mod's keys stay allowed, since UIs and plugins read weather:forecast. No rim.state: script data is already per-mod state. No API version bump, matching the own-events-only change: the docs always said to use your own prefix, and nothing shipped writes another mod's. Keys sort as 'mod:key' in the BTreeMap, so the snapshot can take a mod's data as one range.
