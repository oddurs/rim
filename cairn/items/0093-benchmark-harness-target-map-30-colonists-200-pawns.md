---
id: ee7fe7fd-4cb1-4f65-b23c-2e325c641fdc
title: 'Benchmark harness: target map, 30 colonists, 200 pawns'
type: perf
status: backlog
milestone: scale
depends_on:
- eaf91831-765b-44d0-8ffd-f7c9a49ea44d
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: none
effort: m
layer: tooling
area: perf
pillar:
- performance
---

## Budget

<= 2 ms per tick with 30 colonists and 200 pawns on a 250x250 map.

## Measurement (before)


## Acceptance criteria

- [ ] Reproducible scenario from a seed
- [ ] Reports mean, p99 and per-system times
- [ ] Runs in CI with a regression threshold
