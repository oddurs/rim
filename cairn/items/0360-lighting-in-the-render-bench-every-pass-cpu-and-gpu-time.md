---
id: 360
uid: 6a6dfe88-6d54-49a5-b871-f7eb78f69176
title: 'Lighting in the render bench: every pass, CPU and GPU time'
type: perf
status: done
milestone: lighting
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p0
api: none
effort: s
layer: client
area: perf
---

## Why

Every later lighting ticket trades quality for GPU time, and §6e's numbers are a model. Measure before building, or the presets get tuned by feel. DESIGN.md §6e.

## What

- `rim --bench-render` reports each lighting pass by name with CPU time, draw calls and GPU time. In bench mode only, a pass ends with a one-pixel readback so its GPU time can be taken from the wall clock; normal frames never sync.
- Today's `Sky::light` (lightmap rebuild and multiply) is the first line, recorded here as the baseline.
- A dusk scene: the dense colony at 18:40, 40 static lights, 8 moving, on the 192 × 192 default map and the 250 × 250 stress map.

## Acceptance criteria

- [x] The bench prints a row per lighting pass with CPU, GPU and draw calls
- [x] Baseline for `Sky::light` recorded here and in DESIGN.md §6e
- [x] A frame outside bench mode performs no readback (test)

## 2026-09-26

Baseline, 250x250 bench map, Apple silicon with other builds running (CPU numbers are a range): lightmap rebuild 0.45-1.23 ms CPU, and only when emitters or rooms change (0% of steady frames). Multiply 0.005-0.03 ms CPU, one draw call. GPU per pass comes from GL timer queries (GL_TIME_ELAPSED, via raw GL: miniquad 0.4's ElapsedQuery is a stub whose is_supported() is unimplemented!()), used only where GL_VERSION is desktop 3.3+. On Apple silicon the query times the tile pass it lands in (0.4-10 ms for one full-screen quad), so GPU numbers are read from CI's Linux runner. A first try with glFinish per pass measured vsync on macOS.

## 2026-09-26

Review fixes: lamps now light the colony only for the dusk view, so every other view and the CI budget gate measure the same scene as before. The GPU frames run after each view's capture, so the zooming view's numbers match its frames. The dusk hour settles for 20 ticks, since outdoor terms are evaluated every 20. Timer queries are refused on tile-based GPUs (Apple) rather than reporting tile-pass times, and the bench prints the GL renderer, flagging software GL: CI's llvmpipe 'GPU' times are CPU rasterisation. The rebuild step times only passes that cache, with GPU time, on the view it actually ran on.
