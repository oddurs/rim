---
id: df049dac-deae-4491-99d1-0954a74cd190
title: Roofs from far away, hipped by the span field
type: feature
status: done
milestone: houses
assignee: Oddur Sigurdsson
depends_on:
- 24100bb9-a9f8-430b-9c0a-4b8b7ed4dfb9
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p2
api: none
effort: m
layer: client
area: render
---

## Why

Zoomed out, the colony should read as a village, not a diagram. The roof comes for free from the walls: each cell's distance to the eaves gives a hipped roof on any room shape. DESIGN.md §6c.

## What

- A house is the set of indoor rooms that share walls. Its roof covers their cells and walls.
- Height is the Chebyshev distance to the eaves. Each cell is four triangles shaded by the way they face, from one light.
- The roof material comes from most of the walls: thatch, shingle, turf, slate or tile (`stuff.look.roof`). A hearth gets a chimney.
- Roofs fade in below a zoom level. Hovering or selecting inside a house lifts its roof. A setting pins them on or off.

## Acceptance criteria

- [x] An L-shaped house gets one roof with a valley, not two (screenshot)
- [x] The roof mesh rebuilds only when rooms do
- [x] Render bench within budget with roofs on

## 2026-09-26

Lighting's 153dda59 feeds this roof height field into the occluders and shades the facets by the real sun direction instead of a fixed light. Keep the height field available to the client as data (not only baked into the mesh), so both can read it.

## 2026-09-26

Render bench (loaded machine): the roof pass costs 0.33-0.39 ms on the whole map and about 1 ms mid-zoom. Drawing every roof cell as four triangles and courses cost 4-8 ms whole-map, so past 1500 roofed cells on screen a roof is drawn as rows of runs (one rectangle per run of a house sloping one way) with merged eaves; close in, every cell gets its hips and courses. Roofs::height (one byte a cell) and Roofs::house are kept as data for the lighting pass. Roofs draw after the lightmap, tinted by the outdoor sky.
