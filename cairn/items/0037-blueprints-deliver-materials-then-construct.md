---
id: 37
title: 'Blueprints: deliver materials, then construct'
type: feature
status: done
milestone: castaway
depends_on:
- 35
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: additive
effort: m
layer: engine
area: building
pillar:
- survival
---

## Why

Engine verb: anything with [thing.build] can be placed as a blueprint and built from nearby materials.

## Acceptance criteria

- [x] Pawns fetch the nearest stack and deliver up to carry capacity
- [x] Cancel refunds delivered materials
- [x] Pawns caught inside a finished wall are nudged out

## 2026-09-22

tests/gameplay.rs warrior_chops_and_builds: chop designation + 4 wall blueprints -> walls built within 2 days.
