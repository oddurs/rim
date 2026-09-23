---
id: 30
title: 'Luau host: sandbox, per-script environments, rim.every and rim.on'
type: feature
status: done
milestone: foundations
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: additive
effort: l
layer: engine
area: scripting
pillar:
- plugin-first
- determinism
---

## Why

Behaviour plugins run as sandboxed Luau with deterministic APIs and per-mod timing.

## Acceptance criteria

- [x] math.random and os removed; rim.random draws from the world RNG
- [x] Each script gets its own globals
- [x] World API only valid inside callbacks
- [x] Script errors surface in-game with the mod named
