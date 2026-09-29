---
id: 575
uid: 1b01a342-b38c-4a1d-97f8-bbaff2af3e89
title: Flat lighting smoothed at close zoom, with no extra pass
type: perf
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
effort: s
layer: client
area: render
---

## Why

In the side-by-side (3c65738f) flat's visible cost was blockiness at close zoom: light in hard cell- and room-aligned squares.

## What

- Smooth the upload of the light field (per-corner values from the neighbouring cells, sampled bilinearly) so light eases across cells, in the same one pass, with no extra draw.
- Walls still stop light: no smoothing across a wall or a room's boundary.

## Acceptance criteria

- [ ] Close-zoom shots before and after, the lit hut and dusk included
- [ ] No extra pass or draw call (bench)
