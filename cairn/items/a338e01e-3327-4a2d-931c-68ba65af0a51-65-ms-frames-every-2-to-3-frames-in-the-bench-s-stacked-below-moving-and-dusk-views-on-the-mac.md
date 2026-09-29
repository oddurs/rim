---
id: a338e01e-3327-4a2d-931c-68ba65af0a51
title: 65 ms frames every 2 to 3 frames in the bench's stacked, below, moving and dusk views on the Mac
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
effort: m
layer: client
area: perf
---

## Why

On the Mac (M4 Pro, window in front, 300 frames), medium in the bench's stacked, below, moving and dusk views drops to about 60 fps, with a 62 to 67 ms frame every 2 to 3 frames: 119 to 125 in 300. The glFinish per-pass probe found no lighting pass rerunning in those views, only the multiply every frame. So the spikes aren't a rebake, re-march or readback, and a background run doesn't reproduce them.

## What

- Find what the spikes are: present back-pressure, the dug level's meshes or water, or something else. Start with rapid-cloud's per-frame wall sequence and the views' `rebuilt` counts.
- Fix it, or show it is vsync pacing a frame just over budget.

## Acceptance criteria

- [ ] The cause named, with the measurement that shows it
- [ ] Fixed, or shown to be pacing, with before and after on the Mac
