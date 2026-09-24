---
id: e280b168-ce91-4b09-bd06-91cc34f074b5
title: Allocation-free A* with no corner cutting
type: feature
status: done
milestone: foundations
depends_on:
- f2ea0bd1-a7a0-4b13-84e9-8249f70275e3
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: none
effort: m
layer: engine
area: pathing
pillar:
- performance
- determinism
---

## Why

Pathing is the classic colony-sim hotspot. Searches reuse generation-stamped buffers and never allocate beyond the result.

## Acceptance criteria

- [x] Octile heuristic with integer costs
- [x] Deterministic tie-breaking
- [x] Node cap for pathological searches
