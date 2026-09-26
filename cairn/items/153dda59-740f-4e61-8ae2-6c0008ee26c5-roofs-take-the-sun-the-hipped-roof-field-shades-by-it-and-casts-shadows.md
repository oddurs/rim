---
id: 153dda59-740f-4e61-8ae2-6c0008ee26c5
title: 'Roofs take the sun: the hipped roof field shades by it and casts shadows'
type: feature
status: backlog
milestone: lighting
depends_on:
- 8f4f1de8-5784-4377-8cee-25bcf223275e
- df049dac-deae-4491-99d1-0954a74cd190
created: 2026-09-26
updated: 2026-09-26
priority: p2
api: none
effort: s
layer: client
area: render
---

## Why

Zoomed out, the village is its roofs (df049dac). Their facets are shaded "from one light"; with sky shadows that light should be the sun, and a tall roof should shade the lane beside it. DESIGN.md §6c, §6e.

## What

- The roof height field (Chebyshev distance to the eaves) feeds occluder heights, so a house's shadow has a hipped outline.
- Roof facets shade by their normal against the sun's direction and colour; at night they take the moon's.

## Acceptance criteria

- [ ] An L-shaped house casts one shadow with a valley notch at dusk (screenshot)
- [ ] Roof facets facing the sun are brighter at 09:00 and at 17:00 on opposite sides (screenshots)
