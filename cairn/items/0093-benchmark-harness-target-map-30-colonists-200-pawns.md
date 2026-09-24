---
id: ee7fe7fd-4cb1-4f65-b23c-2e325c641fdc
title: 'Benchmark harness: target map, 30 colonists, 200 pawns'
type: perf
status: done
milestone: scale
assignee: Oddur Sigurdsson
depends_on:
- eaf91831-765b-44d0-8ffd-f7c9a49ea44d
created: 2026-09-22
updated: 2026-09-24
closed_at: 2026-09-24
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

- [x] Reproducible scenario from a seed
- [x] Reports mean, p99 and per-system times
- [x] Runs in CI with a regression threshold

## 2026-09-24

examples/bench.rs: seed-reproducible 250x250 map, 30 colonists, wildlife up to 200 pawns (1 in 8 predators), chop/mine/harvest over the colony's surroundings (or --designate-all for every cell), walls and beds planned; 600 warm-up ticks, then mean/p50/p99/max per tick and each system's mean ms per tick (Profile.totals, exact rather than smoothed). --check fails over the 2 ms budget (3x on CI); CI runs it on Linux for a quarter day. Measured on the reference machine (Apple Silicon, release): default scenario, 1 day: mean 0.238 ms, p50 0.012, p99 8.35, max 37.6 ms, almost all in 'pawns'. --designate-all, half a day: mean 6.67 ms, p50 5.35, p99 23.2, max 43.7, 'pawns' 6.63 ms. The mean is within budget in normal play; the spikes and the designate-all case are find_work scanning, which is what work pools (0cea06b1) addresses.
