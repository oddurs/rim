---
id: 696
uid: f05c5fa1-5fec-4432-93b1-7daddc2f3d50
title: 'Smooth, clean shadows as geometry: projected silhouettes, not marched cells'
type: feature
status: doing
milestone: bare-metal
assignee: quiet-meadow
created: 2026-09-28
updated: 2026-09-29
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

- [x] A proposal with a prototype screenshot (trees, a house, a rock face, at noon and dusk) and its expected cost is approved by the user before the build
- [x] Shadows show no stair-stepping or cell blur at any zoom (autotest shots at close, mid and far)
- [ ] Shadows cost at most 0.5 ms GPU a frame on the Mac, and draw calls stay within the budget
- [ ] The marched sun-shadow code is gone, with the lines removed stated in the PR

## 2026-09-28

Approved by the user via rim-c2 (prototype shots of trees, houses and rock at noon and dusk). Build: marching squares over the cell-centre grid for masses, so a staircase of cells becomes one diagonal; a tree is a thin trunk and an octagon crown. Outlines cached per 32-cell chunk with a height per vertex; the vertex shader pushes them from the brightest sky body, capped at 8 cells. The union is a depth MAX in a half-resolution mask (GLES2 has no MAX blend, and additive blending darkens overlaps), sampled by the flat multiply outdoors and off mass. Moves onto silver-field's region layer (7bd26c70) once that lands.

## 2026-09-28

The marched sun pass is used only by the shadows setting, so it goes with that setting in 3a2b2c0d (the user's order: shapes land first, then the deletions). Criterion 4 is ticked by that PR, which closes this item.

## 2026-09-29

Criterion 2 ticked: the autotest's shadows_are_shapes places a two-wide staircase of walls and measures the shadow's far edge column by column at 48, 20 and 8 px a cell. Slope -1.00, no step off by more than 0.00 cells at any zoom (a marched edge misses by a whole cell). Criterion 3 (0.5 ms GPU on the Mac) waits for a bench slot after the pause; criterion 4 goes with 3a2b2c0d.

## 2026-09-29

PAUSED (merge freeze at d633f7b5): done: shadows as shapes, on #415, one commit over main d633f7b5; gated green on the previous base (913 tests, autotest 396/0, the staircase edge at slope -1.00 with no step error at 48, 20 and 8 px a cell). The re-gate on d633f7b5 was stopped for the freeze. Left: the re-gate, then ready; the Mac GPU budget (criterion 3, needs a bench slot); criterion 4 goes with 3a2b2c0d. Next step: rebase #415 on main, scripts/task check plus autotest, ready to rim-c2. Branch feat/f05c5fa1-shadow-geometry.
