---
id: f5bc43e3-a2a0-4466-8f19-141740804c8f
title: 'Measure: G holds a counting grid with rulers'
type: feature
status: done
milestone: chalkline
assignee: Oddur Sigurdsson
depends_on:
- 553bfb19-5924-4d0a-86f7-a726b6c70d6c
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
priority: p2
api: additive
effort: s
layer: client
area: ui
---

## Why

Counting the cells in a room, or the gap to the river, means hovering cell by cell. DESIGN.md §6f, the Measure level.

## What

- A `core:measure` binding on `g` in `mods/core/ui/keys.luau`, labelled "Measure grid", calling a new `act.toggle_measure()`. The palette finds it.
- At `Measure` the grid draws every line at 15%. Every fifth line, counted from the map's origin, is at `seam_major` and stays at every zoom.
- Each heavier line is numbered where it crosses the pointer's row (above it) and column (beside it), in the UI font on a keyline.
- The pointer's row and column get a 5% chalk wash.

## Acceptance criteria

- [x] `g` toggles Measure, and the command palette lists "Measure grid" (autotest)
- [x] Autotest: at zoom 6 Measure still draws the heavier lines, and Plan draws none
- [x] The heavier lines are numbered along the pointer's row and column (autotest)
- [x] Screenshot `chalk-measure`
- [x] `docs/modding/api-ui.md` lists `act.toggle_measure`

## 2026-09-27

This item adds the theme token seam_major (and its doc row) to mods/core/ui/theme.toml and docs/modding/ui.md: tokens land with the item that first reads them (d83192ed review).

## 2026-09-27

Stacked on box select (463983bb) and the Chalkline stack below it.

## 2026-09-27

Changed from the concept: no ruler strips along the screen's edges, and no coordinate chip. In the game the colonists panel, the inspector and the alerts cover the edges, and the hover readout already names the pointer's cell with its coordinates. The heavier lines are numbered where they cross the pointer's row and column instead, where the eye already is.

## 2026-09-27

Review: Measure is a layer over whatever the tool asks for, not a level of its own, so a tool's lens ticks stay while measuring. G toggles it (DESIGN.md §6f now says so). Every fifth line is drawn over the ordinary line, so it's never fainter than its neighbours while a fade runs. The numbers fade with the lines, thin out when zoomed out so they never run together, and only show over the map. Shadowed map text (stack counts, readouts, numbers) is one helper, overlay::shadowed.

## 2026-09-28

G is now core:measure's, so the guide's sample inspector action and rim_ui's inspector test move their example key from g to y, which core doesn't bind. Without the move, rim_ui's guide_samples_run fails on the key clash. Caught after the PR was readied; it was pulled from the burst and fixed here.
