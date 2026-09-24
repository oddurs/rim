---
id: b4ad855e-8a8a-49ca-8d2e-b7bc8eb2859a
title: 'Keep a removed mod''s data, and give it back when the mod returns'
type: feature
status: backlog
milestone: persistence
depends_on:
- d0524477-919e-4bd6-952e-95ff0d6bb58d
created: 2026-09-22
updated: 2026-09-22
priority: p1
api: none
effort: m
layer: engine
area: save
pillar:
- plugin-first
---

## Why

Removing a mod must not corrupt a save; re-adding it should restore its data.

## What

At the epoch boundary (DESIGN.md §7a), the sections of a mod that is no
longer installed are parked, untouched, in the new root, and written into
every later snapshot. When the mod comes back, they are unparked. Entities
whose def came from the removed mod are dropped, and the load report lists
them.

## Acceptance criteria

- [ ] A removed mod's sections are parked and survive further saves
- [ ] Re-adding the mod restores its data
- [ ] Entities of a removed mod's defs are dropped and listed in a load report
