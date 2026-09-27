---
id: 3124bd7b-9f9a-4d92-84e6-2df736b6e2fe
title: 'Light on every level: buffers keyed by z, sky only down open shafts'
type: feature
status: blocked
milestone: lighting
depends_on:
- 5689930d-2bd1-4838-b403-a72bc61c31e9
- 6fd6b13b-1186-46f4-876e-743d173e03d7
- 8f4f1de8-5784-4377-8cee-25bcf223275e
- acd85584-7f3d-4348-8d56-5242a0bdb620
- e311c029-499c-4764-a4d6-6d1f933f00f9
- f2a8ffc7-9aa8-46cd-9c78-97c3a28001bd
created: 2026-09-26
updated: 2026-09-27
priority: p1
api: none
effort: m
layer: client
area: render
---

## Why

Depth draws one level at a time with the level below through air (DESIGN.md §6d). Each level needs its own light, the sky has to reach down a pit and not into a cellar, and a mine lit by torches should look like one. DESIGN.md §6e.

## What

Light has to be continuous across levels: no edge where one level's light stops, no pop when the view changes level (DESIGN.md §6e, Depth).

- Occluders, static bake and composed light keyed by `z`. The viewed level and its neighbours above and below stay cached; the rest are evicted with their chunk meshes.
- Light crosses openings: each level's compose adds the level above's light through its openings and the level below's through its air cells, read from a blurred half-resolution copy of that level's composed light, attenuated 0.5 per level. Skipped on a level with no openings.
- Sky down a shaft: the levels above count as solid height in the march, so direct sun enters only inside the shaft's cone. Diffuse sky falls with the open sky seen from the bottom, `width / (width + 2·depth)`.
- The level below, through air, is lit by its own light and dimmed by the same depth tint per level.
- Exposure blends between sky-driven and firelight-driven by how much sky reaches the view and eases over a second. Changing level crossfades the two cached buffers for 150 ms.

## Acceptance criteria

- [ ] A torch at the top of a stairwell lights the steps on the level below, fading with distance from the opening, with no visible seam at the opening's edge (screenshot pair, both levels)
- [ ] Sky light down a 3-level shaft falls monotonically with depth; no level is brighter than the one above it (readback test on the composed buffers)
- [ ] Changing level with all three levels cached runs no bake and no occluder rebuild, and mean frame brightness changes by less than 10% per frame during the switch (test)
- [ ] A pit at noon is sunlit on its floor; a cellar beside it is dark (screenshot)
- [ ] Bench on the stacked scene (5689930d) recorded here, inside budget

## 2026-09-26

Agreed with the Depth session. e311c029 keeps every level in one Map's flat arrays (surface is slice 0, then −1…−3, then the levels above; idx(p) includes z). Openings arrive with the portals item acd85584, now a dependency: Map::portals() -> &[Portal { top, bottom }], sorted by top index and bumped in Map::revision; World::air_at(p), where air always opens to (x, y, z−1); and Map::air_cells(z) -> &[u32], kept per level, so compose never scans a level for openings. 5689930d keeps caches for the viewed level ±1.

## 2026-09-27

Portals (acd85584) merged in #225. Still waits on the view (5689930d) and on f2a8ffc7, underground: that item decides what light below ground is, so this starts once it's in review.
