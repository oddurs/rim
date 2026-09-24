---
id: fbf3ee1c-fa3a-4a7c-9c3f-ee09b4398433
title: Spatial index for things by def
type: perf
status: backlog
milestone: scale
depends_on:
- ee7fe7fd-4cb1-4f65-b23c-2e325c641fdc
created: 2026-09-22
updated: 2026-09-22
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
