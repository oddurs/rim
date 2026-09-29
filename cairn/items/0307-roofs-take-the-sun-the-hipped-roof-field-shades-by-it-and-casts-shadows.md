---
id: 307
uid: 153dda59-740f-4e61-8ae2-6c0008ee26c5
title: 'Roofs take the sun: the hipped roof field shades by it and casts shadows'
type: feature
status: done
milestone: lighting
assignee: Oddur Sigurdsson
depends_on:
- 379
- 417
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] An L-shaped house casts one shadow with a valley notch at dusk (screenshot)
- [x] Roof facets facing the sun are brighter at 09:00 and at 17:00 on opposite sides (screenshots)

## 2026-09-27

Done as: a roofed cell's occluder height is its roof's, from Roofs::height (steps in from the eaves): one storey at the eaves, rising ROOF_PITCH 0.5 a cell, so the sun pass casts a hipped shadow. A chunk repacks on a room rebuild when a roof texel changes, not only the roof bit. Roofs::update moved before Light::prepare; it still works only on a room rebuild. Facets shade by the sun: the direct share falls on a slope by n.s against flat ground's, capped at 1.6x, so a flat roof matches the ground. With no sun, the old fixed light from the north-west. The roof tint takes the eye's exposure, as the multiply does for the ground. The drawn drop shadow under roofs is gone: the sun pass casts the real one, and the contact band grounds the walls at night (ruling 0779def9). Chimneys keep their small drawn shadow, since they are not in the occluders. Autotest: an L of walls is one house. At 15 deg from the west, the ground 4 cells past the arm's east wall reads 0.00 (a storey-high box would leave it lit past 3.7 cells), and 7.5 cells out reads 1.00. At zoom 9 with the sun at 25 deg, the east slope reads 0.82 and the west 0.24 in the morning, reversed in the evening. The screenshot shows one L-shaped shadow, longer behind each arm's ridge, with a step where the two arms' shadows meet. Filed 4b6c3a9c: occluders report whole chunks as changed, so a wall near but outside a light's reach still rebakes it.

## 2026-09-27

Review fixes: the facets blend from the fixed north-west light to the sun's by the sun's share (share / DIRECT), so there's no jump at sunset and overcast roofs keep their relief (test). The slope check parks the pointer off the house, so a UI move can't lift its roof mid-check. Declined: pinning --lighting in the autotest, since tests already run the default preset and never read the settings file. A wall top on the far side of the sun is shaded by the roof beside it; that is physically right, and now documented.
