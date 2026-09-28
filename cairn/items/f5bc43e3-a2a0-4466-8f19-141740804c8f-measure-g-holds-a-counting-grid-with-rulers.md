---
id: f5bc43e3-a2a0-4466-8f19-141740804c8f
title: 'Measure: G holds a counting grid with rulers'
type: feature
status: planned
milestone: chalkline
depends_on:
- 553bfb19-5924-4d0a-86f7-a726b6c70d6c
created: 2026-09-27
updated: 2026-09-27
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
- Rulers: a 16 px strip along the top and left edges, labelled at the heavier lines in the UI font. The pointer's cell is marked in chalk on both.
- The pointer's row and column get a 5% chalk wash, and a chip reads "x, y · terrain".

## Acceptance criteria

- [ ] `g` toggles Measure, and the command palette lists "Measure grid" (autotest)
- [ ] Autotest: at zoom 6 Measure still draws the heavier lines, and Plan draws none
- [ ] Screenshot `chalk-measure`
- [ ] `docs/modding/api-ui.md` lists `act.toggle_measure`

## 2026-09-27

This item adds the theme token seam_major (and its doc row) to mods/core/ui/theme.toml and docs/modding/ui.md: tokens land with the item that first reads them (d83192ed review).
