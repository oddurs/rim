---
id: bb171d6e-0759-4c87-b9a6-b1df7ee2ca26
title: 'Pawns by chunk: a spatial index for who is near'
type: perf
status: backlog
milestone: story
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: none
effort: m
layer: engine
area: sim
pillar:
- performance
---

## Why

Witnesses and interaction partners ask "who is near". Today `nearest_pawn`
scans every pawn (DESIGN.md §4g).

## What

Pawns bucketed by chunk and level, updated when a pawn crosses a chunk edge.
Derived state, rebuilt on load. Queries return ids in id order.

## Acceptance criteria

- [ ] Index kept by movement, never rebuilt per tick
- [ ] `near(cell, radius, level)` in id order; `nearest_pawn` uses it
- [ ] Rebuilt on load; a test compares it with a scan after a soak
- [ ] Bench on the target map: query cost and upkeep per tick
