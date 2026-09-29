---
id: 603
uid: 3c65738f-2597-4f74-9007-0732cce58e9e
title: 'Lighting side-by-side: a flat preset against today''s, in screenshots and milliseconds'
type: spike
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
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

- [x] `flat` exists, and its per-frame lighting cost is measured on the Mac
- [x] The side-by-side screenshots and numbers are in the plan doc
- [x] The user has picked A, B or C in the plan, recorded here

## 2026-09-28

Findings so far:
- Blocky at close zoom: yes. flat lights in hard cell- and room-aligned blocks, because stamps fill a room's cells and stop at walls. Bilinear softens one cell's step, not the shape. It's plainest in a lit hut: a hard square round each fire against medium's soft round glow.
- Pictures (seed 1, --background): noon, dusk and hut pairs at close zoom, in the session scratchpad, sent to rim-c2 for the plan doc.
- CPU, same runner (queue run 36489558257, llvmpipe; medium 100 frames, flat 20):
  - The lighting's CPU at medium is under 0.12 ms a frame in every view but moving. There, 64 moving lights cost 3.73 ms against flat's 0.07.
  - llvmpipe's rasterising (submit) falls 0 to 25% under flat: mid 110.8 to 84.3, whole map 72.2 to 70.4.
- Read: flat buys little CPU except on moving lights, which can be fixed inside medium, and it costs the most look. Real-GPU cost awaits the Mac window run, which needs the user's go-ahead.

## 2026-09-28

Correction: medium's 3.73 ms light CPU in the moving view is the moving pass (3.653 ms, one draw of 64 lamp quads, 8 shadowed). On the Mac the same pass is 0.04 to 0.10 ms of CPU. On CI, llvmpipe rasterises the bake shader on the calling thread when the render target changes, so GPU work reads as CPU. The moving lights' real cost is GPU fragment work, not CPU. The lever, if it matters on real GPUs, is moving_shadows or ray steps. So flat's CPU saving is under 0.5 ms a frame in every view, the moving view included.

## 2026-09-28

glFinish per-pass probe on the Mac (background, medium, seed 1; absolute ms inflated by throttling):
- Only the multiply runs every frame, at 1.4–2.9 ms of GPU. The moving pass adds 1.5 ms with 64 moving lights. No bake, sun march, occluder or per-level rebuild reruns in any view, so none causes the hitches.
- Frame sections with the GPU waited for: things (meshes, water, sprites) 5–12 ms, light 2.6–13.6 ms (an upper bound, since it absorbs the previous frame's tail), ui 1.6–7 ms.
- The background run doesn't reproduce the slow-last-four views. A likely reading: medium's multiply pushes views near the 8.3 ms 120 Hz budget over vblank, so 60 fps. The 62–67 ms spikes are unexplained by lighting passes and need a foreground run or rapid-cloud's per-frame walls.
- Lever for option A: compose the light at light resolution once, and multiply by a single bilinear read, instead of about 11 texture reads per screen pixel.

## 2026-09-28

The user picked option B (via rim-c2, 2026-09-28): flat is the default lighting, and today's look (medium, high, ultra) is opt-in, labelled with its cost. Follow-ups filed: flat as the default, flat smoothed at close zoom, opt-in tiers composing at light resolution, and the 65 ms spikes as a bug of their own.

## 2026-09-28

Done: flat is built and measured on the Mac (rapid-cloud: 120 fps in every view, 0 hitches). The side-by-side pictures and numbers are in the user's plan doc (rim-c2), and the user picked option B. The follow-up is 08a5d182.
