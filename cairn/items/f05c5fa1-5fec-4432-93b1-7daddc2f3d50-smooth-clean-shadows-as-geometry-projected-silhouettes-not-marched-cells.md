---
id: f05c5fa1-5fec-4432-93b1-7daddc2f3d50
title: 'Smooth, clean shadows as geometry: projected silhouettes, not marched cells'
type: feature
status: backlog
milestone: bare-metal
assignee: quiet-meadow
created: 2026-09-28
updated: 2026-09-28
priority: p0
api: none
effort: l
layer: client
area: render
---

## Why

The user, 2026-09-28: "the angular blurred cell shadows that trail the trees houses mountains etc are just awful. make sure that gets addressed. shadows are always performance first but the game needs some really smooth shadow action with clean aesthetic." Today's sun shadows are marched on the light grid, so they come out blocky and blurred along cell edges, and they cost GPU every frame (see the Mac numbers under Bare Metal).

## What

- A shadow is geometry, not a lighting pass. Every caster (tree, wall, roof, rock face) has a silhouette with a height, baked into the chunk and region meshes when the chunk is built, as its own region layer.
- A vertex shader projects each silhouette along the sun's direction and length, taken from the sim's body state (#328 and #339). A moving sun changes one uniform and rebuilds nothing.
- The edge is analytic or SDF anti-aliased: crisp at the base and softer with distance, one tone, no marching and no cell stairs. The moon casts the same way at night, and cloud fades both.
- The shadow layer costs about 4 to 16 draw calls on the whole map, joined over regions. The fragment cost is one alpha blend.
- Once it's in, the marched sun shadows and anything only they use are deleted.

## Acceptance criteria

- [ ] A proposal with a prototype screenshot (trees, a house, a rock face, at noon and dusk) and its expected cost is approved by the user before the build
- [ ] Shadows show no stair-stepping or cell blur at any zoom (autotest shots at close, mid and far)
- [ ] Shadows cost at most 0.5 ms GPU a frame on the Mac, and draw calls stay within the budget
- [ ] The marched sun-shadow code is gone, with the lines removed stated in the PR
