---
id: 8d8bad33-511f-441e-b8ec-f9b0b03732db
title: 'A whole-frame bench: CPU, GPU, present, draw calls and hitches, on the Mac with the window in front'
type: perf
status: doing
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
- [x] The HUD shows the live frame budget in the running game
- [ ] Today's Mac baseline is recorded (every bench view, lighting medium), and CI's two runner classes are named in the bench output

## 2026-09-28

First PR. rim --bench-render adds a whole-frame table per view: the wall time from one frame's start to the next (median, p99, worst), hitches (frames over 2x the median) and rest (the mean frame less every pass, submit and gpu: the present and waits); the JSON gets the same per view and a machine field. It names the machine (CPU and threads) so CI's two Linux runner classes are told apart. Vsync is off for the bench (swap_interval 0 under --bench-render; one line in conf(), agreed with calm-forest, main.rs's owner); macOS paces frames anyway, so there rest holds the wait. The HUD: a frames.rs ring of the last second (median, worst, the client's own mesh and figure calls, the biggest pass), written four times a second, shown first in the F3 profiler through a new view.frame(); main.rs gets a field, its default, one call at the end of render() and one field in client_view(). Shown in the autotest's profiler shot: 'frame 13.8 ms, worst 193.1 · 11 calls · biggest ui 1.68 ms' (relabelled 'mesh and figure calls' since, as macroquad's own batches aren't counted without the bench's capture). scripts/bench-front runs the bench in front; docs/engineering/benchmarks.md says why a background run measures nothing.
