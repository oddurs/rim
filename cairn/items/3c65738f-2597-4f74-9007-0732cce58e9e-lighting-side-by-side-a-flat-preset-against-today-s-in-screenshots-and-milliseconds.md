---
id: 3c65738f-2597-4f74-9007-0732cce58e9e
title: 'Lighting side-by-side: a flat preset against today''s, in screenshots and milliseconds'
type: spike
status: backlog
milestone: bare-metal
assignee: quiet-meadow
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
