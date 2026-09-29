---
id: ecdb6295-d123-4ef4-a1a2-9802168cd463
title: 'Panels rest in their corners: slots sit on their edge, one column width, a fixed hover readout'
type: feature
status: backlog
milestone: rimos
created: 2026-09-29
updated: 2026-09-29
priority: p1
effort: m
layer: client
area: ui
api: none
---

## Problem

Playing, the colonist inspector (bottom left) and the hover readout (bottom right) don't sit in their corners. The user wants everything to have its place.

- Slots pin to the top: `min_h` with `justify: Start` (crates/rim_ui/src/lib.rs:1795-1806). A short inspector or hover panel floats at the top of its reserved 300 or 132 px, not on the dock.
- The left column is ragged: colonists are 234 wide (mods/core/ui/people.luau:218), the inspector 300 (inspector.luau:41).
- The hover readout has no fixed width (maxw 460, hud.luau:62-94), so it changes size as its contents change, beside Now panels of 300.
- `SHEET_SIDE` is 280 (lib.rs:255), but the columns are 300 + 16, so at 1280 px wide a sheet overlaps the inspector by about 36 px (lib.rs:663-672).
- The tray is placed from the left column's edge, not centred (lib.rs:1450-1459).

## Proposal

Bottom slots rest on their edge (`justify: End`), with one gutter everywhere. The left column has one width, and the hover readout a fixed width that matches the right column. Sheets start after the columns. The tray is centred.

## Acceptance criteria

- [ ] An autotest shot shows a short inspector and hover readout resting on the dock, one gutter from the screen edge
- [ ] The colonists and inspector share one width, and the hover readout keeps one width as the hovered thing changes
- [ ] No sheet overlaps a side column at 1280 px wide
