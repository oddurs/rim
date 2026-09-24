---
id: 96d2dac9-cf4e-409a-a028-f49902c8d7d9
title: 'Chunks: one dirty unit for regions, fields and the renderer'
type: perf
status: backlog
milestone: scale
created: 2026-09-24
updated: 2026-09-24
priority: p2
api: none
effort: m
layer: engine
area: map
pillar:
- performance
---

## Why

Incremental region updates (0097), render caching (0098) and field
re-stamping all want the same thing: a fixed-size chunk with a dirty bit.
DESIGN.md §6a makes it the one structural addition to `map.rs`, so the
three systems share a unit instead of each inventing its own.

## What

- A chunk grid over the map (size chosen by measurement; 16 or 32).
- `Map` tracks dirty chunks per concern (passability, terrain, fields)
  alongside the existing `revision` and `changed` cells.
- Consumers ask for dirty chunks and clear their own flag.

## Budget

Placing a wall dirties one chunk. No per-tick cost when nothing changes.

## Acceptance criteria

- [ ] Dirty chunks exposed on `Map` and consumed by at least one system
- [ ] Determinism test passes
- [ ] Benchmark before and after recorded here
