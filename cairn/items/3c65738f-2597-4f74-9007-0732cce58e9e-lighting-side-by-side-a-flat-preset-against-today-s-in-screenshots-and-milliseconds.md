---
id: 3c65738f-2597-4f74-9007-0732cce58e9e
title: 'Lighting side-by-side: a flat preset against today''s, in screenshots and milliseconds'
type: spike
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
claimed: 2026-09-28
created: 2026-09-28
updated: 2026-09-28
priority: p0
api: none
effort: m
layer: client
area: render
---

## Why

The user finds the lighting slow and ugly. Choosing to keep it, simplify it or rebuild it needs today's stack and a lite path side by side, both in the picture and in the frame cost.

## What

- A `flat` preset below low. The lightmap is the sim's `light` field, uploaded only when it changes and tinted by the sky and firelight colours. One multiply draw, with no marches, bakes or room fill.
- Check first whether a cell-sized light field reads as blocky at close zoom, and whether bilinear upsampling fixes it.
- Every bench view under `flat` and under `medium` (Mac, window in front), plus paired screenshots at noon, at dusk and in a lit hut, all put into the user's plan doc.

## Acceptance criteria

- [ ] `flat` exists, and its per-frame lighting cost is measured on the Mac
- [ ] The side-by-side screenshots and numbers are in the plan doc
- [ ] The user has picked A, B or C in the plan, recorded here

## 2026-09-28

Findings so far:
- Blocky at close zoom: yes. flat lights in hard cell- and room-aligned blocks, because stamps fill a room's cells and stop at walls. Bilinear softens one cell's step, not the shape. It's plainest in a lit hut: a hard square round each fire against medium's soft round glow.
- Pictures (seed 1, --background): noon, dusk and hut pairs at close zoom, in the session scratchpad, sent to rim-c2 for the plan doc.
- CPU, same runner (queue run 36489558257, llvmpipe; medium 100 frames, flat 20):
  - The lighting's CPU at medium is under 0.12 ms a frame in every view but moving. There, 64 moving lights cost 3.73 ms against flat's 0.07.
  - llvmpipe's rasterising (submit) falls 0 to 25% under flat: mid 110.8 to 84.3, whole map 72.2 to 70.4.
- Read: flat buys little CPU except on moving lights, which can be fixed inside medium, and it costs the most look. Real-GPU cost awaits the Mac window run, which needs the user's go-ahead.
