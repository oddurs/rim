---
id: 981e6792-492d-4ece-9d1f-8875312eb3b3
title: 'Scaling bench: tick and frame cost against colony and map size'
type: perf
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
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

- [ ] `bench --scale` prints the ladder and each system's growth exponent
- [ ] Every system growing faster than linear is filed as its own bare-metal item
- [ ] Per-size budgets are proposed to the user from the measured curves
