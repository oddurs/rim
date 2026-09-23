---
id: 61
title: Serialize world, pawns, things and jobs
type: feature
status: backlog
milestone: persistence
depends_on:
- 60
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: none
effort: l
layer: engine
area: save
pillar:
- determinism
---

## Why

Save and load a running colony exactly.

## Acceptance criteria

- [ ] Load then run gives the same state hash as never saving
