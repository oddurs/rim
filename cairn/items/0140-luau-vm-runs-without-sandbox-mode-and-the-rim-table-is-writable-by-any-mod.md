---
id: 3eb7e697-6bb4-4318-90f4-7f4d727b98c7
title: Luau VM runs without sandbox mode, and the rim table is writable by any mod
type: bug
status: backlog
milestone: plugin-api
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: breaking
effort: s
layer: engine
area: scripting
pillar:
- plugin-first
---

## What happens

`ScriptHost::load` builds the VM with `Lua::new()` and never enables Luau sandbox mode. The `rim` global is a plain table, so any mod can overwrite `rim.spawn_pawn` (or anything else) for every other mod. Item 0030 was closed as "sandbox"; what actually exists is removed `os` and `math.random`, and per-script globals.

## What should happen

A mod from an unknown GitHub repo is safe to run, and no mod can rewrite the engine API for the others (DESIGN.md §10).

## Reproduction

Seed: any
Mods: core plus a script containing `rim.spawn_pawn = function() end`
Tick: any

1. Load both; core's incidents silently stop spawning pawns.

## Acceptance criteria

- [ ] Luau sandbox mode enabled; audit of remaining globals (`debug`, `getfenv`/`setfenv`, `load`) recorded in this item
- [ ] Engine `rim` table frozen; writing to it raises an error that names the mod
- [ ] Test: a hostile script can neither reach I/O nor alter another mod's API
