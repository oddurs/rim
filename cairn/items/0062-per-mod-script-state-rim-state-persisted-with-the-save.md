---
id: 65f0b723-5310-4dc6-a0f6-ba1223edb25b
title: 'Per-mod script state: rim.state persisted with the save'
type: feature
status: backlog
milestone: persistence
depends_on:
- c5d185be-9bb8-491d-b73d-94f75e4018f0
created: 2026-09-22
updated: 2026-09-24
priority: p0
api: additive
effort: m
layer: engine
area: scripting
pillar:
- plugin-first
---

## Why

The storyteller's memory (last incident, tension) must survive a reload.

## Acceptance criteria

- [ ] Each mod gets a serializable state table

## 2026-09-24

Core's storyteller now keeps last/last_threat in script data (core:storyteller) instead of Luau locals, so the memory is hashed and will be saved; tested in mods/core/tests/storyteller.luau. What remains here: whether mods need a rim.state table beyond set_data's namespaced keys, and the save/load itself (0061).
