---
id: 96
uid: fbf3ee1c-fa3a-4a7c-9c3f-ee09b4398433
title: Spatial index for things by def
type: perf
status: done
milestone: scale
depends_on:
- 93
created: 2026-09-22
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: none
effort: m
layer: engine
area: sim
pillar:
- performance
---

## Budget

Finding the nearest wood must not scan every thing.

## Measurement (before)


## Acceptance criteria

- [ ] Per-def buckets by chunk
- [ ] find_work and find_food use it

## 2026-09-26

Delivered by the stock ledger and holdings item (ac643c1f): holdings by item def and chunk is this index. Close it when that item lands.

## 2026-09-26

Done by the stock ledger item (ac643c1f): holdings by item def and chunk, used by the nearest-item search.
