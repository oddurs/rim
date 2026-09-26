---
id: 236d1860-820d-4025-b742-9a95eb1231ac
title: Sheets float at fixed proportions, and don't jump when the side columns change
type: bug
status: done
milestone: interface
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: none
effort: m
layer: engine
area: ui
---

## Why

A sheet (Work, Stockpiles, News) is placed in the band between the side columns as last frame laid them out. The tile readout on the right is wider than the Now panels, so hovering the map widens the right column, narrows the band, and the Work sheet slides over a frame later. Anything transient on a side moves the screen the player is working in.

## What

- A sheet's place comes from the screen and its own declared size alone: centred, at most 60% of the width and the height between the bars, set a little above centre. Nothing docked moves it.
- Windows get a soft shadow, and a sheet a short open (fade and rise, 140 ms, drawn by offsetting the draw list, no relayout).
- Escape closes an open sheet after dropping the tool and before clearing the selection.

## Acceptance criteria

- [x] Hovering the map, opening a palette or selecting something never moves or resizes an open sheet
- [x] At 1280 and 1600 wide, a sheet leaves the people column and the Now column uncovered
- [x] The open animation costs no layout pass and stays within the UI budget
- [x] Escape closes the open sheet

## 2026-09-26

Placement uses the screen, the declared size and fixed reservations (280 each side, 48 top, 56 bottom), never last frame's docked layout. The stay-put test fails on main with 'the readout moved the sheet'. Watch out: a test run with CARGO_TARGET_DIR shared across checkouts linked a stale rim_ui; clean the package before trusting results.
