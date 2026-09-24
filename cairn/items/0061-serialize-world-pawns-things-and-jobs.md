---
id: c5d185be-9bb8-491d-b73d-94f75e4018f0
title: Serialize world, pawns, things and jobs
type: feature
status: backlog
milestone: persistence
depends_on:
- 70edf863-73a1-4a4d-b284-84e9803d2252
created: 2026-09-22
updated: 2026-09-24
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

## 2026-09-24

Format per DESIGN.md §7a: named components, qualified def ids, save-local entity numbers, derived state rebuilt, CBOR vs MessagePack + zstd to be measured here (size and load time on a year-old colony). Add the round-trip test: save at tick N, load, run M ticks; state hashes must equal the unsaved run's, in the crosscheck scenario on every platform.
