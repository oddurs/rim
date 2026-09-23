---
id: 95
title: Flow fields for raid groups
type: perf
status: backlog
milestone: scale
depends_on:
- 93
created: 2026-09-22
updated: 2026-09-22
priority: p2
api: none
effort: m
layer: engine
area: pathing
pillar:
- performance
---

## Budget

One field per raid target instead of N searches.

## Measurement (before)


## Acceptance criteria

- [ ] Raids of 30 pathing cost less than 5 A* searches
