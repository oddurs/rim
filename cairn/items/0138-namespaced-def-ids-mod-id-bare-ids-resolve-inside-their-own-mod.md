---
id: be8174f0-ff41-44fe-b788-2ffff60e0d19
title: 'Namespaced def ids: mod:id, bare ids resolve inside their own mod'
type: feature
status: done
milestone: persistence
assignee: Oddur Sigurdsson
created: 2026-09-23
updated: 2026-09-24
closed_at: 2026-09-24
priority: p0
api: breaking
effort: m
layer: engine
area: modding
pillar:
- plugin-first
- determinism
---

## Why

With many mods, two will both add `iron`. Saves key on def ids, so this has to land before the save format or it breaks every save later. See DESIGN.md §10.

## What

Every def is addressed as `mod:id` (`core:wolf`). Inside a mod, a bare id means that mod's own def; any other mod's def needs the prefix. Patch targets use the same form: `thing/core:berry_bush`.

## Acceptance criteria

- [x] Loader qualifies every def id with its mod id
- [x] Bare references resolve within the defining mod; cross-mod references need the prefix, with an error that suggests it
- [x] Two mods may define the same bare id without conflict
- [x] Luau APIs accept and return qualified ids
- [x] core and wildlife_plus migrated; api version bumped

## 2026-09-24

Every def id is stored qualified (core:wall); the loader prefixes bare ids and rejects defining in another mod's namespace. References resolve in the def's home mod (its id's prefix), in finalize; scripts resolve against calling_mod (the api! macro gained a |w, from, args| arm) and the UI against ui_calling_mod. A bare id that only another mod defines fails with the prefix to use, for defs, patch targets and script calls alike; before, a mistyped patch target was a silent skip. DefDb::lookup stays lenient for tools and tests (qualified, or a bare id exactly one mod defines), DefDb::resolve is the strict rule. Tool keys and UI node ids built from def ids are now qualified (build:core:wall, core:stuff.core:wood); autotest screenshot names replace ':' with '_' for Windows and CI artifacts. Subtle: patch match/remove compare values as the target def wrote them (core writes field = "temperature"), documented in patches.md. API 0.3.
