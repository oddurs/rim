---
id: 98
title: Chunked terrain render caching
type: perf
status: backlog
milestone: scale
created: 2026-09-22
updated: 2026-09-22
priority: p1
api: none
effort: m
layer: client
area: render
pillar:
- performance
---

## Budget

Terrain draws from cached chunk textures.

## Measurement (before)


## Acceptance criteria

- [ ] Chunks redraw only when their revision changes
