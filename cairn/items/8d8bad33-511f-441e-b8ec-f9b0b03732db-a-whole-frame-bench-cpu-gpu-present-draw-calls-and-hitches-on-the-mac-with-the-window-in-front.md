---
id: 8d8bad33-511f-441e-b8ec-f9b0b03732db
title: 'A whole-frame bench: CPU, GPU, present, draw calls and hitches, on the Mac with the window in front'
type: perf
status: backlog
milestone: bare-metal
assignee: rapid-cloud
created: 2026-09-28
updated: 2026-09-28
priority: p0
api: none
effort: m
layer: client
area: perf
---

## Why

CI gates only world CPU, 0.4 to 2.6 ms, while submit costs 47 to 77 ms and GPU time, draw calls and hitches go unchecked. The user feels the game is slow, and no number shows it. A hidden window on macOS is throttled, so a background bench measures nothing.

## What

- `rim --bench-render` reports the whole frame: CPU passes, GL submit, GPU (timer queries where GL has them, Metal's frame time on the Mac if reachable), present, and hitches (frames over 2× the median).
- A frame-time HUD in the game (F-key toggle): the last second's median and worst frame, draw calls, and the biggest pass.
- A scripted Mac run that brings the window to the front for the bench, with a note in docs on why a background run is useless.
- A baseline table of today's numbers on the Mac, filled into the plan doc.

## Acceptance criteria

- [ ] The bench prints whole-frame p50/p99, submit, GPU and hitches per view
- [ ] The HUD shows the live frame budget in the running game
- [ ] Today's Mac baseline is recorded (every bench view, lighting medium), and CI's two runner classes are named in the bench output
