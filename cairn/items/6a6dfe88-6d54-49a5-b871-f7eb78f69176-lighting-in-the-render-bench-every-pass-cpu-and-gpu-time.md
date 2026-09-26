---
id: 6a6dfe88-6d54-49a5-b871-f7eb78f69176
title: 'Lighting in the render bench: every pass, CPU and GPU time'
type: perf
status: backlog
milestone: lighting
created: 2026-09-26
updated: 2026-09-26
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

- [ ] The bench prints a row per lighting pass with CPU, GPU and draw calls
- [ ] Baseline for `Sky::light` recorded here and in DESIGN.md §6e
- [ ] A frame outside bench mode performs no readback (test)
