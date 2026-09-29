---
id: 2dff88de-ef34-47b9-92fe-fab10e75caa7
title: Rows scrolled out of a scroll area still block the map under them
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: none
layer: client
area: ui
---

## What

Every node with a background or a handler inside a scroll area registers a
"solid" rectangle at its **unscrolled, unclipped** layout rect
(`crates/rim_ui/src/lib.rs` ~1605, the `collect_nodes` pass pushes `r`
straight into `lo.solids`). Hits are scrolled and clipped by `paint::walk`;
solids are not.

## How it fails

A scroll area whose content is taller than the area (every managed window's
body is a scroll area in kit.luau; the command palette's list; the inspector's
work list; the dock tray) leaves its hidden rows' rects hanging below the area,
over the map. In `Ui::route` a point inside any solid sets `over_solid`, so:

- a left or right click on the map there is captured by the UI and does nothing;
- `mouse_over_ui` is true there, so the world hover and the cursor hint vanish;
- `Ui::covers` says the UI covers it (overlay.rs uses it for chevrons).

Rows scrolled *up* out of view leave solids at their old places too.

## Repro

Open the command palette (Ctrl+K): its ~70 rows run far below the 360 px
window. Click the map just under the window: nothing is selected. Same for any
window resized shorter than its content.

## Fix

Carry the scroll offset and the clip through `collect_nodes` as `paint::walk`
does, and register a solid only for the visible part. `Ui::find` keeps
returning layout rects (documented, and the autotest relies on it).
