---
id: 310
uid: 17009725-a061-4445-829f-b1c73c6ce2bd
title: 'Memories: the event log indexed by person, thing, place and date'
type: feature
status: backlog
milestone: story
depends_on:
- 76
- 354
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: m
layer: engine
area: sim
pillar:
- growth
- performance
---

## Why

The chronicle's store is the one memory mechanism: what a person lived through,
what a knife remembers, what happened in a room (DESIGN.md §4g).

## What

Index the chronicle's records by entity, place (room, cell) and day. A memory
kind like `belonged_to` is a record that links a thing to a person.

## Acceptance criteria

- [ ] `rim.memories({ entity =, room =, day =, kind = })` returns records newest first, with a limit
- [ ] Lookups are indexed, never a scan of the log
- [ ] Mods add memory kinds as data and write records with `rim.remember`
- [ ] Records name entities by stable id; a despawned entity keeps its name in the record
- [ ] Retention by notability, bounded in bytes; saved as its own section
