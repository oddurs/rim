---
id: c9973f7f-b39d-487b-bf80-68183fc880e7
title: Storyteller as a Luau plugin with a shared incident registry
type: feature
status: done
milestone: castaway
depends_on:
- aaaae5e7-462e-449e-832a-d6631b5f840b
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
