---
id: ca786a8e-09cf-49ef-8003-1b49468a16a4
title: 'Mod save migrations: a migrate hook when a mod''s version changes'
type: feature
status: done
milestone: persistence
assignee: Oddur Sigurdsson
depends_on:
- d0524477-919e-4bd6-952e-95ff0d6bb58d
- 65f0b723-5310-4dc6-a0f6-ba1223edb25b
created: 2026-09-23
updated: 2026-09-24
closed_at: 2026-09-24
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
because a mod's version changed (DESIGN.md §7a), the function it registered
with `rim.on_migrate` runs before any hook, over its own script data, on the
world being loaded. The epoch commits only if every migration succeeds.

## Acceptance criteria

- [x] Saves record mod id and version for every mod
- [x] Migrate hook sees only the calling mod's data
- [x] A failed migration is reported with the mod named, and the save does not load half-migrated
- [x] Test: save with a v1 fixture mod, load with v2, data migrated

## 2026-09-24

A registration, rim.on_migrate(fn), not an export: exports are per-script tables with no agreed entry point, and a registration matches rim.every/rim.on. fn(from_version, data) gets the mod's script data by bare key and returns what to keep; ScriptHost::migrate swaps the mod's keys for the result. Snapshot::restore_noting calls it for every mod whose version differs from the snapshot's lock, in load order, after the world is rebuilt and before anything steps; an error fails the whole load (the save file writes its new epoch only after a successful load, so the file is untouched). Component sections for mods don't exist yet (7f8ce379); when they do, they belong in data too.

## 2026-09-24

Review: a mod removed and later re-added skipped its migration, since the save's lock no longer listed it; its data now carries the version it was written with (World::data_versions, in engine:world) and it migrates from there. Migrators get no world (rim.random, set_data and the rest error), so a migration is a pure function of the mod's data. on_migrate is refused after load, an empty key is refused, and the docs say 'a different version', since a downgrade migrates too.
