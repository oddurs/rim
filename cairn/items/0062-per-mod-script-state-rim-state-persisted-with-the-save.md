---
id: 65f0b723-5310-4dc6-a0f6-ba1223edb25b
title: 'Per-mod script state: rim.state persisted with the save'
type: feature
status: backlog
milestone: persistence
depends_on:
- c5d185be-9bb8-491d-b73d-94f75e4018f0
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
