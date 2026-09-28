---
id: b276d529-40cf-4617-9b62-fc5528c3258d
title: 'no_clocks fails on main: the growth and fire tests time their passes'
type: bug
status: done
milestone: proving-ground
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p0
api: none
effort: s
layer: tooling
area: tests
---

## Problem

#262 (growth) and #270 (fire) merged in the same burst as #273, which added the no_clocks guard. Each adds a timed assert: the plant pass under 3 ms (growth.rs) and a fire step under 10 ms (fire.rs). Main fails rim_sim::no_clocks from 31aacea4.

## Proposal

- grow() returns the plants it worked out, like spoil(); the test counts the staggering.
- A Luau fire step has no work to count from outside, so its budget is gated on RIM_BUDGETS and listed in CI's budget step.

## Acceptance criteria

- [x] no_clocks passes on the branch, rebased on main
- [x] The plant-pass test fails with the stagger removed
- [x] The fire budget passes under RIM_BUDGETS=1, and CI's budget step runs it

## 2026-09-27

no_clocks and growth and fire pass (13 tests). With the stagger removed the plant-pass test fails: [6120, 6120, 6120, 6120]. Under RIM_BUDGETS=1 alone: one step with 289 cells burning 3.3 ms, budget 30.
