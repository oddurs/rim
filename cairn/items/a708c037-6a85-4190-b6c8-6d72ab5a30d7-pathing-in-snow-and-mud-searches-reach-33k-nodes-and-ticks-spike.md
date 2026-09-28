---
id: a708c037-6a85-4190-b6c8-6d72ab5a30d7
title: 'Pathing in snow and mud: searches reach 33k nodes and ticks spike'
type: perf
status: backlog
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
effort: m
layer: engine
area: pathing
---

## Why

Measured on main 2026-09-28 with bench --haul --days 0.25 (8d551753 setup), at load 43-56: path searches reached 8,000-33,000 nodes and made 50-127 ms ticks. With snow and mud's move_cost lines (0192) removed from the defs, pawns went from 0.22 to 0.14 ms a tick and p99 from 8-11 to 2-3 ms. The haul search itself is 0.003-0.012 ms a tick, so this is pathing. The numbers are noisy under load; confirm on a quiet machine first.

## Acceptance criteria

- [ ] Measured on a quiet machine, main against move_cost removed, and noted here
- [ ] p99 tick with move_cost within 1.5x of without on the same setup
- [ ] The determinism test passes
