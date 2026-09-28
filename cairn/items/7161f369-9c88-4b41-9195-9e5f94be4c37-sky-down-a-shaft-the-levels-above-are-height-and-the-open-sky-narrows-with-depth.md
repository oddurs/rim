---
id: 7161f369-9c88-4b41-9195-9e5f94be4c37
title: 'Sky down a shaft: the levels above are height, and the open sky narrows with depth'
type: feature
status: backlog
milestone: lighting
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
effort: m
layer: client
area: render
depends_on:
- 3124bd7b-9f9a-4d92-84e6-2df736b6e2fe
---

## Why

A pit dug at noon should have a sunlit floor, and a shaft three levels deep should darken smoothly toward its bottom, not light every level like the surface or none of them. Split from 3124bd7b, which gives each level its own light but none of the sky's (DESIGN.md §6e, Depth).

## What

- A cell below the surface sees the sky only through a column of air to the surface. In the sun march, the levels above count as solid height, so direct sun enters only inside the shaft's cone.
- The diffuse sky share falls with the open sky seen from the bottom, `width / (width + 2·depth)`.

## Acceptance criteria

- [ ] Sky light down a 3-level shaft falls monotonically with depth; no level is brighter than the one above it (readback test)
- [ ] A pit at noon is sunlit on its floor; a cellar beside it is dark (screenshot)
