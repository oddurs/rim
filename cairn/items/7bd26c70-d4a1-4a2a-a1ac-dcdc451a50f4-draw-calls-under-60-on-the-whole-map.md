---
id: 7bd26c70-d4a1-4a2a-a1ac-dcdc451a50f4
title: Draw calls under 60 on the whole map
type: perf
status: backlog
milestone: bare-metal
assignee: silver-field
created: 2026-09-28
updated: 2026-09-28
priority: p0
api: none
effort: l
layer: client
area: render
---

## Why

The whole map costs 129 to 139 draw calls and about 1.3 million indices a frame. Every call is fixed overhead on integrated GPUs and in GL submit.

## What

- Build on #340's batched pawns: batch overlays, UI and things by material and texture, and drop draws of what can't be seen.
- Report calls and indices for each pass in the bench.

## Acceptance criteria

- [ ] The whole map runs at 60 draw calls or fewer (bench)
- [ ] No view's world + ui time gets worse on the same runner class
