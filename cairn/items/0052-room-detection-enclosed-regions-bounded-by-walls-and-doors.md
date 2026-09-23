---
id: 52
title: 'Room detection: enclosed regions bounded by walls and doors'
type: feature
status: backlog
milestone: shelter
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: additive
effort: m
layer: engine
area: map
pillar:
- survival
- performance
---

## Why

Rooms are regions that do not touch the map edge. Recomputed only when walls change.

## Acceptance criteria

- [ ] Rooms exposed to scripts
- [ ] Doors split regions for rooms but not for pathing
