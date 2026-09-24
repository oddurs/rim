---
id: fecf1c87-0065-4f0c-a59b-f3c10a0e6b24
title: Custom needs with script satisfiers
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: additive
effort: m
layer: engine
area: needs
pillar:
- plugin-first
---

## Why

Mood, recreation, comfort all need needs the engine does not know about.

## Acceptance criteria

- [ ] satisfier = "script" calls a registered function
- [ ] The `Satisfier` enum becomes a registry: core registers food and rest like any other mod would
