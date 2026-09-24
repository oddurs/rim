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

From the save-format decision (DESIGN.md §7a): the Luau VM isn't saved, so any state a mod must keep lives in script data (rim.set_data: saved and hashed). Today core's storyteller keeps last/last_threat in Luau locals, which a load would reset and the desync check can't see. Move it to script data, and consider whether a per-mod rim.state table is still needed on top of set_data's namespaced keys.
