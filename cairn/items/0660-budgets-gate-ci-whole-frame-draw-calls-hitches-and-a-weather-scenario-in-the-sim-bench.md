---
id: 660
uid: a1fe6816-88bf-41a0-bef7-ac3741032db8
title: 'Budgets gate CI: whole frame, draw calls, hitches, and a weather scenario in the sim bench'
type: perf
status: doing
milestone: bare-metal
assignee: rapid-cloud
created: 2026-09-28
updated: 2026-09-28
priority: p0
api: none
effort: m
layer: tooling
area: perf
---

## Why

The budgets in the Bare Metal milestone mean nothing unless CI fails when they're broken. Today the sim bench has no weather, which is how 50 to 127 ms ticks from snow and mud went unseen (a708c037).

## What

- The render bench fails the queue lane over budget on the whole frame, draw calls or hitches, not only world CPU. Budgets are scaled per runner class.
- The sim bench gains a winter scenario with snow, mud and hauling, gated on mean, p99 and max tick.
- A PR run posts the numbers it moved.

## Acceptance criteria

- [ ] The queue lane fails when a planted regression breaks a frame, draw-call or hitch budget
- [ ] The sim bench's weather scenario runs in CI and fails a planted slow path search
- [x] The budgets live in one data file that the bench and the docs both read

## 2026-09-28

Part A (the sim). budgets.toml at the repo root holds each budget's target (the goal) and cap (what CI fails over now; only ever lowered). rim_sim::budgets reads it, names the machine, and scales time caps (_ms) by the runner class's slack on CI; counts of work aren't scaled. The sim bench gains --winter (a thaw at 1 C: 20 cm of snow over ground at 95% wetness, where mud starts costing, plus the hauling case) and gates --check on budgets.toml's sim.base or sim.winter: mean, p99 and max tick, and two counts, the most path nodes in a tick and nodes per search. Counts because times on this shared Mac swung 2-3x between two runs of one deterministic scenario (winter's worst tick 56 ms, then 4.6 ms), while its node counts didn't move (36,903 most in a tick, 91 per search). scripts/task sim runs both. Criterion 2 shown locally: a zero A* heuristic planted in path.rs failed sim.winter.nodes_per_search (190 over its cap of 100) while every time measure stayed within its cap; reverted. Part B, after #350: the render bench's gates and the docs table read from budgets.toml. Finding for stream 4: winter's worst ticks include 45-56 ms spent in the pawns system with no path search at all.

## 2026-09-28

Correction to the note above: the 45-56 ms 'most in pawns' ticks with no search were load, not code. A second run's worst winter tick was 4.6 ms, and quiet-field saw the same vanish at lower load. What the thaw does show is even-cost floods: snow and mud make nearly every step about 1.8x what A*'s heuristic assumes, a different mechanism from a708c037's lake floods (quiet-field, 2026-09-28).

## 2026-09-28

Part B. budgets.toml gains render.gpu and render.software: world CPU on the gated views, the whole frame's p99 and worst, hitches and draw calls, each the worst view's; the bench picks the section by its GL renderer. gpu caps hold today's Mac (M4 Pro, medium: lit views 14-17 ms with 65 ms hitches) and carry the plan's targets; software caps hold today's CI llvmpipe on AMD EPYC 9V74 (frame 70-142 ms, 130-140 calls, no hitches). That class's slack is 1.0: the software caps are set on it and it plays the sim as fast as the M4 Pro; others keep 3. rim --bench-render --check reads it and exits 1 over any cap; the old 4 ms constant is gone. docs/engineering/benchmarks.md's Budgets table is written from budgets.toml by tests/budgets_doc.rs (RIM_UPDATE_DOCS=1 rewrites it), which fails when they differ (criterion 3).

## 2026-09-28

Also --sync (asked for by rim-c2): glFinish before each present, so a frame's wall time is its CPU and GPU cost with nothing queued. On the Mac, where GL has no timer, it tells a GPU-bound view (the lit views' 14-17 ms median with 65 ms frames every two or three) from a real hitch.
