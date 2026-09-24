---
id: 3eb7e697-6bb4-4318-90f4-7f4d727b98c7
title: Luau VM runs without sandbox mode, and the rim table is writable by any mod
type: bug
status: done
milestone: plugin-api
assignee: Oddur Sigurdsson
created: 2026-09-23
updated: 2026-09-24
closed_at: 2026-09-24
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

- [x] Luau sandbox mode enabled; audit of remaining globals (`debug`, `getfenv`/`setfenv`, `load`) recorded in this item
- [x] Engine `rim` table frozen; writing to it raises an error that names the mod
- [x] Test: a hostile script can neither reach I/O nor alter another mod's API

## 2026-09-24

Fixed in two PRs. #19: the sim VM loads only math, string, table, bit32, utf8 and buffer (no os, io, debug, coroutine); removes collectgarbage, gcinfo, loadstring, getfenv, setfenv, newproxy and math.random; makes the libraries and globals read-only; each script runs in a private safe environment. Audit of what remains: base functions (pairs, ipairs, pcall, error, tostring, select, type, rawget/rawset/rawequal, setmetatable/getmetatable, print) plus rim; none reach I/O, the clock, memory stats or another mod's environment. This PR: mods see rim through a proxy. While mods load a write may add a key but never replace one ("mod 'rogue' can't replace rim.spawn_pawn: it belongs to the engine"); after load rim and every table in it are read-only; __metatable hides the proxy. Tests in tests/scripting.rs: replacing an engine function, another mod's API (rim.register_incident, rim.weather), or the metatable fails to load with both mods named; writes after load fail; escape hatches are absent.
