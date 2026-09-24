---
id: 0e73145a-39c4-4f1d-88a4-59d803a2f535
title: 'Work pools: work givers post work, pawns stop scanning'
type: perf
status: backlog
milestone: colony
depends_on:
- 03ad3e3a-efe1-4186-b764-8dcd1d9344f1
created: 2026-09-24
updated: 2026-09-24
priority: p0
api: additive
pillar:
- growth
effort: m
layer: engine
area: ai
---

## Budget

Choosing work under 0.2 ms a tick at 200 pawns on a stress map with every cell designated (DESIGN.md §4d).

## Measurement (before)

`find_work` in `ai.rs` scans every blueprint, designation and creature for each idle colonist. Record its cost on the stress map before starting.

## Approach

- A pool per work type. Work givers post and withdraw work on events (designated, blueprint placed, item left outside a stockpile) instead of being polled.
- Pools bucketed by reachability region and chunk: unreachable work is rejected in O(1), nearby work is found without a full scan.
- Each pawn caches its work types in effective-priority order, rebuilt only when the grid, a rule or a stance changes.
- Inside a priority level, an integer score: distance, urgency (rot, fire, bleeding) and skill fit.
- The top few candidates and why each lost are kept only for inspected pawns.

## Acceptance criteria

- [ ] Benchmark shows the budget is met, before and after recorded here
- [ ] `find_work`'s scans are gone; blueprints, designations and hunts post to pools
- [ ] Determinism test passes; a pool's order never depends on hash iteration

## 2026-09-24

Measurement (before), from the benchmark harness (examples/bench.rs, reference machine, release): cargo run --release -p rim_sim --example bench -- --days 0.5 --designate-all gives mean 6.67 ms per tick, p50 5.35, p99 23.2, max 43.7, with 'pawns' at 6.63 ms of it (seed 1, 250x250, 29 colonists, 199 pawns). Without --designate-all (a colony-sized area designated), 1 day: mean 0.238 ms but p99 8.35 ms and max 37.6 ms, also all 'pawns'.
