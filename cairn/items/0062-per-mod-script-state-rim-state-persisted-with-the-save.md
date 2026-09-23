---
id: 62
title: 'Per-mod script state: rim.state persisted with the save'
type: feature
status: backlog
milestone: persistence
depends_on:
- 61
created: 2026-09-22
updated: 2026-09-22
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
