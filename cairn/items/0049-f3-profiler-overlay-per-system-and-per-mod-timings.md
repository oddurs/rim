---
id: 49
title: 'F3 profiler overlay: per-system and per-mod timings'
type: feature
status: done
milestone: castaway
depends_on:
- 45
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p1
api: none
effort: s
layer: client
area: perf
pillar:
- performance
- plugin-first
---

## Why

Make every system's and every mod's cost visible from day one.

## Acceptance criteria

- [x] Smoothed microseconds per system
- [x] Per-mod script time
- [x] Pathfinder searches and nodes expanded

## 2026-09-23

Verified by rim --autotest (crates/rim_client/src/autotest.rs), which drives the real client through the same Actions keyboard and mouse produce, checks state and saves screenshots; 56/56 checks pass, screenshots reviewed by eye. Runs in CI on macOS. Profiler lists tick, pawns, needs, regions, rooms, wealth and mod:core timings, plus pathfinder searches/nodes, entity counts and load order.
