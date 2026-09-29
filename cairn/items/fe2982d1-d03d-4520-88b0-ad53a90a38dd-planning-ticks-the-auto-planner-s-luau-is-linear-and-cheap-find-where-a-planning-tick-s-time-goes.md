---
id: fe2982d1-d03d-4520-88b0-ad53a90a38dd
title: 'Planning ticks: the Auto planner''s Luau is linear and cheap; find where a planning tick''s time goes'
type: perf
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
effort: m
layer: core
area: ai
---

## Budget

A planning tick stays inside the 4 ms hitch budget at 100 colonists.

## Measurement

Main c06bedc0, bench --colonists N --pawns 250 --days 0.5 (12 planning calls), at load 76–128, so every wall-clock figure is rough: per call about 0.17 ms at 10 colonists, 0.33 at 28, and 1.25–21 ms at 100 across repeats, with single planning ticks of 65–330 ms at 100. The spread says load.

The deterministic count says the Luau is cheap. Steps counted by the VM's interrupt (a call or a loop back-edge), per planner call on the bench's boards: 369–509 at 10 colonists, 1,015–1,369 at 30, 3,429–4,379 at 100. That's linear, about 35 steps a colonist.

## What was tried

`auto.luau` fills each missing person on a work type by scanning every member, which is O(work × colonists²) when need is high: lots waiting against a small `per_person`. A rewrite kept candidates in heaps, one per work type and count of Firsts, and gave exactly the old plans on 100+ random boards. The equivalence test caught a changed LOAD and a reversed tie order. But counted in steps, planner plus board generation in a bare VM, 11 work types:

| colonists | normal board old / new | every work 1000 waiting, per_person 1: old / new |
|---|---|---|
| 10 | 1,916 / 2,534 | 2,422 / 3,439 |
| 30 | 7,028 / 8,167 | 9,365 / 11,539 |
| 100 | 51,141 / 39,981 | 75,041 / 61,476 |
| 150 | 101,163 / 64,560 | 153,438 / 99,370 |

The heaps cost more below about 50 colonists, the design target being 30, and save only 1.2–1.6x at 100–150. Even the worst case is about 150k steps a call, once a game hour. Not worth the code, so it wasn't kept.

## What's left

If planning ticks really cost milliseconds, the time isn't in the planner's Luau. Candidates: building the board (Rust to Luau tables, colonists × work types × 3), reading and applying the plan, and the GC work the call's allocations trigger. Split `planner:` in the profiler into board, call, read and apply, and measure on a quiet machine before changing anything.

## Acceptance criteria

- [ ] The planning tick's time split into board, call, read and apply, measured on a quiet machine at 30 and 100 colonists
- [ ] No planning tick over 4 ms at 100 colonists, or a filed follow-up naming the part that is
