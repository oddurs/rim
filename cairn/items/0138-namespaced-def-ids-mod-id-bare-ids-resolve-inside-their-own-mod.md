---
id: be8174f0-ff41-44fe-b788-2ffff60e0d19
title: 'Namespaced def ids: mod:id, bare ids resolve inside their own mod'
type: feature
status: backlog
milestone: persistence
created: 2026-09-23
updated: 2026-09-23
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

- [ ] Loader qualifies every def id with its mod id
- [ ] Bare references resolve within the defining mod; cross-mod references need the prefix, with an error that suggests it
- [ ] Two mods may define the same bare id without conflict
- [ ] Luau APIs accept and return qualified ids
- [ ] core and wildlife_plus migrated; api version bumped
