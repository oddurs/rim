---
id: 3124bd7b-9f9a-4d92-84e6-2df736b6e2fe
title: 'Light on every level: buffers keyed by z, sky only down open shafts'
type: feature
status: backlog
milestone: lighting
depends_on:
- 5689930d-2bd1-4838-b403-a72bc61c31e9
- 6fd6b13b-1186-46f4-876e-743d173e03d7
- 8f4f1de8-5784-4377-8cee-25bcf223275e
- e311c029-499c-4764-a4d6-6d1f933f00f9
- f2a8ffc7-9aa8-46cd-9c78-97c3a28001bd
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: none
effort: m
layer: client
area: render
---

## Why

Depth draws one level at a time with the level below through air (DESIGN.md §6d). Each level needs its own light, the sky has to reach down a pit and not into a cellar, and a mine lit by torches should look like one. DESIGN.md §6e.

## What

- Occluders, static bake and composed light keyed by `(z)`; the viewed level and the one below cached, the rest evicted with their chunk meshes.
- Sky reaches level `z` only where the column above is open to the sky; a pit gets sun with its rim's shadow.
- The level below, through air, is drawn with its own composed light, dimmed.
- Underground, exposure follows the firelight in view, not the sky.

## Acceptance criteria

- [ ] Changing level with both levels cached runs no bake and no occluder rebuild (test)
- [ ] A pit at noon is sunlit on its floor; a cellar beside it is dark (screenshot)
- [ ] Bench on the stacked scene (5689930d) recorded here, inside budget
