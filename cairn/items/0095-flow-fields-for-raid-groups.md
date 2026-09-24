---
id: a353667b-9f31-4c7b-8da0-cf6f46b9c9f0
title: Flow fields for raid groups
type: perf
status: backlog
milestone: scale
depends_on:
- ee7fe7fd-4cb1-4f65-b23c-2e325c641fdc
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
