---
id: 1c626720-942b-43da-ac89-618a170d2623
title: 'Custom namespaced events: rim.emit and rim.on("mod:event")'
type: feature
status: done
milestone: plugin-api
assignee: Oddur Sigurdsson
created: 2026-09-23
updated: 2026-09-24
closed_at: 2026-09-24
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

- [x] `rim.emit("<own mod>:<name>", table)` queues an event; emitting under another mod's namespace is an error
- [x] Handlers run in deterministic order (load order, then registration order), at the same point as engine events
- [x] Payloads are plain data (no functions), so they can be saved and replayed
- [x] Engine events keep bare names (`pawn_died`); mod events are always prefixed

## 2026-09-24

rim.emit existed since the Weather sprint (0182); now namespaced: a mod may only emit "<its id>:<name>", and the id is taken from the chunk name of the calling Luau code, so when mod B calls rim.weather.force the weather plugin's code emits weather:changed. Emitting another mod's namespace or a bare engine name is an error. Handlers run in load order then registration order, when engine events do; payloads go through script data (plain data only). The weather plugin's event is now weather:changed. Test: mods_emit_only_their_own_events.
