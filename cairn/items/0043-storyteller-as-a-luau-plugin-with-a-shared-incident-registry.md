---
id: 43
title: Storyteller as a Luau plugin with a shared incident registry
type: feature
status: done
milestone: castaway
depends_on:
- 30
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: additive
effort: m
layer: core
area: storyteller
pillar:
- wealth-gravity
- plugin-first
---

## Why

The storyteller is content, not engine. It exposes rim.register_incident so other plugins can add incidents.

## Acceptance criteria

- [x] Incident pull scales with wealth
- [x] Threat points weigh wealth against colony strength
- [x] Grace period before first raid
- [x] Other mods register incidents without touching core files
