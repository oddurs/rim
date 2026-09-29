---
id: 636
uid: 7161f369-9c88-4b41-9195-9e5f94be4c37
title: 'Sky down a shaft: the levels above are height, and the open sky narrows with depth'
type: feature
status: done
milestone: lighting
assignee: Oddur Sigurdsson
depends_on:
- 329
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p2
api: none
effort: m
layer: client
area: render
---

## Why

A pit dug at noon should have a sunlit floor, and a shaft three levels deep should darken smoothly toward its bottom, not light every level like the surface or none of them. Split from 3124bd7b, which gives each level its own light but none of the sky's (DESIGN.md §6e, Depth).

## What

- A cell below the surface sees the sky only through a column of air to the surface. In the sun march, the levels above count as solid height, so direct sun enters only inside the shaft's cone.
- The diffuse sky share falls with the open sky seen from the bottom, `width / (width + 2·depth)`.

## Acceptance criteria

- [x] Sky light down a 3-level shaft falls monotonically with depth; no level is brighter than the one above it (readback test)
- [x] A pit at noon is sunlit on its floor; a cellar beside it is dark (screenshot)

## 2026-09-28

Done as: below the surface, a cell with air over it to the surface is open, and packs as what stands in it. A covered cell packs under a roof, depth storeys tall (rock still stops fire), so a ray from a shaft climbs past the levels above, and the sun reaches in only inside the shaft's cone. Rock under rock is roofed too, or its top (the surface) took the sun. A chunk below repacks when the same chunk above changes. The rooms texture's G is the open sky a cell sees: 1 on the surface, width/(width+2 depth) in a shaft (width the narrower of its runs), 0 under rock. The multiply scales the sky's ambient by it. The sun pass runs on every level again. Autotest at a clear noon, pits 1, 2 and 3 levels deep: (open, sun) = (0.60, 1.00), (0.43, 1.00), (0.33, 0.00), falling with depth. The one-deep pit's floor is sunlit, and the cellar of the stacked scene reads (0, 0). The per-level check allows the pits' sky: level -1 reads 0.07 to the surface's 0.32, sun 0 under rock.

## 2026-09-28

Review fixes:
- Covered rock stays a mass, for its contact shadow. It's marked kind 3 (COVERED), and the sun pass leaves such a cell dark rather than roofing it.
- Doors and windows below keep their firelight leak.
- A shaft is closed by a floor laid over its air, or by a roof over its mouth at the surface. Chunks below repack on fixtures above as well as terrain.
- Below the surface the eye adapts to the dark, whatever the sky up top. A sealed mine is no darker at noon than at night. The blend on changing level is 220a059e.
- A level below with no open sky runs no sun pass.
- The autotest's pits stay on the map and are filled back in after.
Declined: open_sky's cost, which is O(open cells x run) and reruns only when a level above changes.
