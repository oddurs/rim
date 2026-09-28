---
id: 4b6c3a9c-edfe-4bed-859b-67f0ca5f8df3
title: Occluders report changed cells, not whole chunks, so a far wall rebakes nothing
type: perf
status: done
milestone: lighting
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p3
api: none
effort: s
layer: client
area: render
---

## Budget

A wall built where no light reaches bakes no firelight. Only a wall within a light's reach redoes that light.

## Measurement (before)

`Occluders::update` reported each repacked chunk (32×32 cells) as changed, and the bake redoes every light whose reach touches a changed rectangle. So a wall 20 cells from a fire, but in a chunk that reaches it, rebaked the fire although nothing it lights changed. Found while making the indoors autotest's far-wall check robust (2f13e01d): that check had to pick a wall whose whole chunk was out of every light's reach.

## Approach

- `pack` tracks the bounding box of the texels that actually changed in each chunk, and `changed` reports those boxes rather than the chunk.
- The sun pass is unaffected: it reruns whole on any occluder change.

## Acceptance criteria

- [x] A wall placed just beyond a light's reach, in a chunk that reaches the light, redoes no firelight (autotest)
- [x] `changed` is, per repacked chunk, the box round the cells whose texels changed (test)

## 2026-09-27

Done as: pack keeps the bounding box of the texels that changed in each repacked chunk, and reports that instead of the chunk; a whole repack still reports whole chunks. Partial texture uploads shrink with it. Tests: a tree set down reports its own cell alone, and closing a hut reports boxes inside the hut. Autotest: a wall just past a stove's reach, in a chunk its light reaches, bakes nothing (0 bakes). The check picks its wall so that the old chunk rule would have redone that light; I didn't run it against the old code.

## 2026-09-27

Review follow-up: each box reaches a cell past the changed texels, since the passes sample the occluders filtered and a changed texel reaches half a cell past its own. Tests updated to match.
