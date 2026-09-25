---
id: 96d2dac9-cf4e-409a-a028-f49902c8d7d9
title: 'Chunks: one dirty unit for regions, fields and the renderer'
type: perf
status: done
milestone: graphics
assignee: Oddur Sigurdsson
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
priority: p0
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

- [x] Dirty chunks exposed on `Map` and consumed by at least one system
- [x] Determinism test passes
- [x] Benchmark before and after recorded here

## Measurement

Sim bench (`--days 0.25`, Apple M-series): mean tick 0.030 ms before,
0.028 after; p99 0.564 → 0.520. Within noise: a touch is an add or two.

## 2026-09-24

Revisions, not dirty bits: Map keeps a revision per chunk per concern (terrain, things) and consumers remember what they last saw. The client never writes to the world, and any number of consumers can read without clearing each other's flags. Concerns with a consumer only: terrain (the ground texture, which now re-uploads one chunk instead of the whole map on any fixture change) and things (the chunked meshes, next). Passability waits for incremental regions (0097), and field re-stamping has no consumer yet. Size 32: 64 chunks on the §8 map; 16 would be 256 draw calls for the meshes. The meshes item can measure and change CHUNK. Changes the map can't see (counts, designations, regrowth) call Map::touch / World::touch; tests/chunks.rs plays a busy colony and fails if a chunk's drawn state changes without its revision.
