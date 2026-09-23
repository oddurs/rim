---
id: 139
title: 'Mod save migrations: a migrate hook when a mod''s version changes'
type: feature
status: backlog
milestone: persistence
depends_on:
- 62
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

The save records each mod's version. On load, if a mod's version differs, its `migrate(from_version, state)` export runs before any hook, over its own `rim.state` and entity components.

## Acceptance criteria

- [ ] Saves record mod id and version for every mod
- [ ] Migrate hook sees only the calling mod's data
- [ ] A failed migration is reported with the mod named, and the save does not load half-migrated
- [ ] Test: save with a v1 fixture mod, load with v2, data migrated
