---
id: 482
uid: 553bfb19-5924-4d0a-86f7-a726b6c70d6c
title: 'The grid: Rest, Lens and Plan, drawn as a groove under things'
type: feature
status: done
milestone: chalkline
assignee: Oddur Sigurdsson
depends_on:
- 533
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] A unit test covers `grid_level` for every tool with and without a drag
- [x] Autotest: with the wall tool at zoom 28, screenshots `chalk-grid-lens` and `chalk-grid-plan` (mid-drag) differ from `chalk-grid-rest` near the pointer, measured with `block_diff`
- [x] Autotest: at zoom 8 the Plan screenshot equals the Rest screenshot
- [x] Autotest: a wall cell's pixels are the same with the grid on and off
- [x] Autotest at night (22:00): the Plan grid still differs from Rest near the pointer
- [x] `rim --bench-render --check` passes, and the whole-map views add no draw calls

## 2026-09-27

This item adds the theme token seam (and its doc row) to mods/core/ui/theme.toml and docs/modding/ui.md: tokens land with the item that first reads them (d83192ed review).

Started on a branch stacked on d83192ed (palette), which is in review; it rebases onto main once that merges.

## 2026-09-27

Two criteria changed while building. A tree sways on the wall clock, so its pixels never match between shots; the check uses a wall, which never moves. The night check runs in the autotest's existing night section at 22:00, which is fully dark. The grid needs world_ui's previews to follow the frame's input pointer (app.pointer), not the OS cursor, or the autotest's drag preview lands wherever the real mouse is; in play they are the same point. Precipitation is pinned to 0 for the section, since falling rain moves pixels.

## 2026-09-27

Review: kept seams adding up where lines cross; the design wants vertices a little darker, like a survey grid. Changed: the grid now sits above floors (between mesh layers 0 and 1) so a floored room still shows it; a click is not a drag (app.dragged, shared with world_ui's preview); a line is one point rounded to whole target pixels so it doesn't shimmer at render scale 0.5; the lens deepens only the seam.

## 2026-09-27

Criterion 6: the local bench ran over budget on every view at load average 250, grid or not (hover's branch did the same). Main's CI bench is the judge. By construction, the whole-map views are below GRID_MID_ZOOM, so grid::draw returns before drawing anything there.
