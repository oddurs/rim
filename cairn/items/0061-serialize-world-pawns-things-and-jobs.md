---
id: c5d185be-9bb8-491d-b73d-94f75e4018f0
title: Serialize world, pawns, things and jobs
type: feature
status: backlog
milestone: persistence
depends_on:
- 70edf863-73a1-4a4d-b284-84e9803d2252
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
