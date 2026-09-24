---
id: ca786a8e-09cf-49ef-8003-1b49468a16a4
title: 'Mod save migrations: a migrate hook when a mod''s version changes'
type: feature
status: backlog
milestone: persistence
depends_on:
- d0524477-919e-4bd6-952e-95ff0d6bb58d
- 65f0b723-5310-4dc6-a0f6-ba1223edb25b
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: additive
effort: m
layer: engine
area: save
pillar:
- plugin-first
- determinism
---

## Why

Mods on GitHub ship updates. A save made with `mood 0.3` must load with `mood 0.4`, or players stop updating, and authors stop changing their data layout.

## What

The save's lockfile records each mod's version. When a new epoch begins
because a mod's version changed (DESIGN.md §7a), its `migrate(from_version,
sections)` export runs before any hook, over its own script data and
component sections, on a staging copy. The epoch commits only if every
migration succeeds.

## Acceptance criteria

- [ ] Saves record mod id and version for every mod
- [ ] Migrate hook sees only the calling mod's data
- [ ] A failed migration is reported with the mod named, and the save does not load half-migrated
- [ ] Test: save with a v1 fixture mod, load with v2, data migrated
