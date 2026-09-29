---
id: 8eb8a76b-3f28-4c98-b284-a49a3f04fb65
title: A mod's grid with huge rows or cols aborts the client
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p3
api: none
layer: client
area: ui
---

## What

`node_from_table` (crates/rim_ui/src/node.rs, the grid branch) takes `rows`
and `cols` from the mod's table unchecked (`num(..).max(0.0) as usize`) and
reserves `Vec::with_capacity(rows * cols)` before calling `cell(r, c)` once.

## How it fails

`ui.grid({ rows = 1e6, cols = 1e6, ... })` asks for 10^12 cells up front:
the allocation fails and the process aborts (`handle_alloc_error`), so the
UI's error box and the call deadline never get a chance. Past `usize` the
product overflows (a panic in debug builds). A UI mod is meant to fail into
an error box with the rest of the UI running (DESIGN.md §11).

## Fix

A grid has at most a fixed number of cells (far above the Work Board's
colonists by work types); more is a node error naming the limit, shown in
the mod's error box. The product is checked, so it can't overflow.
