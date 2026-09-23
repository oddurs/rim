---
id: 142
title: 'Custom namespaced events: rim.emit and rim.on("mod:event")'
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: additive
effort: s
layer: engine
area: scripting
pillar:
- plugin-first
- determinism
---

## Why

Soft coupling between mods: a mood mod reacts to a stampede without depending on wildlife_plus. Today only six engine events exist and mods can't add any.

## Acceptance criteria

- [ ] `rim.emit("<own mod>:<name>", table)` queues an event; emitting under another mod's namespace is an error
- [ ] Handlers run in deterministic order (load order, then registration order), at the same point as engine events
- [ ] Payloads are plain data (no functions), so they can be saved and replayed
- [ ] Engine events keep bare names (`pawn_died`); mod events are always prefixed
