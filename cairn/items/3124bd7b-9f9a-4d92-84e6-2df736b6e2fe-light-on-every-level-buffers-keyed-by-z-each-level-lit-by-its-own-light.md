---
id: 3124bd7b-9f9a-4d92-84e6-2df736b6e2fe
title: 'Light on every level: buffers keyed by z, each level lit by its own light'
type: feature
status: done
milestone: lighting
assignee: Oddur Sigurdsson
depends_on:
- 5689930d-2bd1-4838-b403-a72bc61c31e9
- 6fd6b13b-1186-46f4-876e-743d173e03d7
- 8f4f1de8-5784-4377-8cee-25bcf223275e
- acd85584-7f3d-4348-8d56-5242a0bdb620
- e311c029-499c-4764-a4d6-6d1f933f00f9
created: 2026-09-26
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: none
effort: m
layer: client
area: render
---

## Why

Depth draws one level at a time with the level below through air (DESIGN.md §6d). Each level needs its own light, the sky has to reach down a pit and not into a cellar, and a mine lit by torches should look like one. DESIGN.md §6e.

## What

- Occluders, the firelight bake, the sun and the rooms keyed by `z`. The viewed level and its neighbours above and below stay cached; the rest are evicted with their chunk meshes.
- Each level is lit by its own lights. Below the surface, no sky reaches: the sky down a shaft is 7161f369, light across openings 1104bf12, and the crossfade on changing level 220a059e.
- The multiply runs on every level, where the view (5689930d) draws a level below unlit today.

## Acceptance criteria

- [x] Each level is lit by its own fires, and a level below the surface with none is dark (autotest readback)
- [x] Changing level with all three levels cached runs no bake and no occluder rebuild (test)
- [x] Bench on the stacked scene (5689930d) recorded here, inside budget

## 2026-09-26

Agreed with the Depth session. e311c029 keeps every level in one Map's flat arrays (surface is slice 0, then −1…−3, then the levels above; idx(p) includes z). Openings arrive with the portals item acd85584, now a dependency: Map::portals() -> &[Portal { top, bottom }], sorted by top index and bumped in Map::revision; World::air_at(p), where air always opens to (x, y, z−1); and Map::air_cells(z) -> &[u32], kept per level, so compose never scans a level for openings. 5689930d keeps caches for the viewed level ±1.

## 2026-09-27

Portals (acd85584) merged in #225. Still waits on the view (5689930d) and on f2a8ffc7, underground: that item decides what light below ground is, so this starts once it's in review.

## 2026-09-28

Split: this item keeps the per-level buffers and each level's own light. The sky down a shaft went to 7161f369, light across openings to 1104bf12, and the crossfade and exposure on changing level to 220a059e, since each is a PR of its own. It no longer waits on f2a8ffc7 (underground warmth): quiet-field confirms that item is a temperature field, and light below ground being zero is what this item gives the renderer.

## 2026-09-28

Done as: everything the light keeps for a level (occluders, firelight bake and moving lights, sun target, rooms and fill) moved into a Level, keyed by z. The one in view is Light::lv, levels one away stay cached, and further ones are let go. Occluders pack level z's chunks. Lamps, fill and room shares read level z. Below the surface the sun key is Down, the sky's light is zero, room shares are zero, and night's floor is a quarter of the surface's (UNDERGROUND). The multiply runs on every level, and the view's z == 0 gate went (agreed with quiet-field); roofs stay surface-only. Autotest, on the view section's stacked scene: in full daylight up top, level -1 reads 0.07 to the surface's 0.32, with sun 0. A campfire below lights its level at 0.79 and the surface at 0.00. Switching between the cached levels ran 0 bakes, 0 sun passes and no repack. Criterion 3 (bench) comes from CI's stacked and below views, now lit.

## 2026-09-28

Bench, from main's CI on 5a2f5477 (Linux llvmpipe, 100 frames), with the levels lit: the stacked view draws the world in 0.585 ms mean and the below view in 0.369 ms, inside the 4 ms budget. Their multiply is 0.012 and 0.015 ms of CPU, and the other lighting passes did no work on steady frames. The render check passed.
