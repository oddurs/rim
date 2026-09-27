---
id: 553bfb19-5924-4d0a-86f7-a726b6c70d6c
title: 'The grid: Rest, Lens and Plan, drawn as a groove under things'
type: feature
status: planned
milestone: chalkline
depends_on:
- d83192ed-0a3e-4a6f-a39a-04ce94a68935
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: none
effort: m
layer: client
area: render
---

## Why

Every order is a promise about cells, and there is no grid (DESIGN.md §6a). Showing one all the time turns the colony into graph paper, so it appears only while a tool is in hand. DESIGN.md §6f.

## What

- `overlay::grid_level(tool, dragging, measure) -> Level`, where `Level` is `Rest`, `Lens`, `Plan` or `Measure`. With the select tool it is `Rest`, or `Lens` while a box drag is under way. With a tool armed it is `Lens`, and `Plan` during a drag.
- A grid pass drawn in `draw::things` right after the ground quad and before `meshes.draw_layer`, so standing things hide it:
  - Seam: 1 px `seam` on the cell boundary. Lit edge: 1 px chalk at 4.5%, one pixel right of and below the seam.
  - `Lens` draws a plus at each corner (arm 2–4 px, chalk 45% over a seam), fading out over `LENS_CELLS = 5.5` cells from the pointer with a smoothstep.
  - `Plan` draws every line, plus 18% more seam within the lens.
- Zoom bands: no lines or ticks below `GRID_MID_ZOOM = 10` points a cell, 70% strength below `GRID_NEAR_ZOOM = 20`, and full strength from there.
- Fades: 160 ms up, and 280 ms down after a 400 ms hold.
- The grid is drawn before lighting, so it darkens with the ground. If a midnight screenshot shows it gone, move the lit edge after lighting, and record which way it went on this item.

## Acceptance criteria

- [ ] A unit test covers `grid_level` for every tool with and without a drag
- [ ] Autotest: with the wall tool at zoom 28, screenshots `chalk-grid-lens` and `chalk-grid-plan` (mid-drag) differ from `chalk-grid-rest` near the pointer, measured with `block_diff`
- [ ] Autotest: at zoom 8 the Plan screenshot equals the Rest screenshot
- [ ] Autotest: a tree cell's pixels are the same with the grid on and off
- [ ] Autotest at midnight: the Plan grid still differs from Rest near the pointer
- [ ] `rim --bench-render --check` passes, and the whole-map views add no draw calls
