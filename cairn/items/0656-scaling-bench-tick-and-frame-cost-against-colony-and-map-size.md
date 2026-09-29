---
id: 656
uid: 981e6792-492d-4ece-9d1f-8875312eb3b3
title: 'Scaling bench: tick and frame cost against colony and map size'
type: perf
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
claimed: 2026-09-28
created: 2026-09-28
updated: 2026-09-29
priority: p0
api: none
effort: m
layer: tooling
area: perf
---

## Why

The user: "performance is a key metric in this game. I MEAN BARE METAL SPEED. It's our number one feature. this game runs beautifully and scalably." Every bench today runs one scene (250x250, about 200 pawns), so nothing shows how cost grows with a colony or a map. Something that is O(n²) in pawns or cells looks fine at 200.

## What

- The sim and render benches run a ladder of sizes: pawns (for example 50, 200, 800, 3,200) and maps (for example 128², 250², 500², 1000²). Report mean, p99 and max tick, frame time, and draw calls at each size.
- A fitted growth exponent per system (pawns, fields, water, paths, rooms, render passes). Anything above linear is flagged by name.
- The results go into the budgets (per-size ceilings) once the user agrees the sizes.

## Acceptance criteria

- [x] `bench --scale` prints the ladder and each system's growth exponent
- [x] Every system growing faster than linear is filed as its own bare-metal item
- [ ] Per-size budgets are proposed to the user from the measured curves

## 2026-09-28

Sim half built: bench --scale pawns (50, 200, 800, 3,200 on 500²) or --scale map (128², 250², 500², 1000² with 200 pawns), each rung the bench in a process of its own writing --report JSON, then each system's mean ms per tick at every rung and its growth exponent (log-log least-squares slope; above 1.15 flagged). First run, M4 Pro at load 91-115, 0.1 days a rung: against map cells, regions 1.90, rooms 1.83, boundary 1.65, spoil 1.49, mod:primitive 1.34, shelter 1.31, grow 1.28, and the whole tick 0.27 to 6.2 ms from 128² to 1000²; against pawns, deaths 1.28, the tick 2.3 to 4.5 ms. Filed: 489acd07 rooms, c1199842 boundary, 1985507c spoil, 89812fe6 mod:primitive, 6994b4ce shelter, eb8ec1f7 grow, affb8d41 deaths; regions noted on the existing 66906291. The render ladder (rim --bench-render --scale) and per-size budgets for the user come next: the render ladder needs a window in front.

## 2026-09-29

PAUSED: done: the sim bench's --scale ladder (this branch, in the gate queue) and its findings filed (#375). Left: the render ladder (rim --bench-render --scale, pawns and map sizes, needing --size and --pawns in bench::world and the same rung-per-process runner) and criterion 3, per-size budgets proposed to the user from both curves. Next step: after a1fe6816-B merges (both touch bench.rs), build the render ladder, ask rim-c2 for a window-in-front slot, run it, then write per-size caps into budgets.toml as a proposal for the user. Branch: perf/981e6792-scaling-bench for the sim half; the render half starts fresh from main.
