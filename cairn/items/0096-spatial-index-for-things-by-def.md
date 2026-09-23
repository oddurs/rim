---
id: 96
title: Spatial index for things by def
type: perf
status: backlog
milestone: scale
depends_on:
- 93
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
