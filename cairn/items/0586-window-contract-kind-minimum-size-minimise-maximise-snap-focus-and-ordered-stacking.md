---
id: 586
uid: 2adfa3ac-b19d-41b3-bbfb-eeb18b7f3191
title: 'Window contract: kind, minimum size, minimise, maximise, snap, focus and ordered stacking'
type: feature
status: backlog
milestone: rimos
depends_on:
- 564
- 594
- 380
created: 2026-09-28
updated: 2026-09-28
priority: p0
api: additive
effort: l
layer: client
area: ui
---

## Problem

`ui.window` drags, resizes, closes and raises, and remembers positions. There is no minimise, maximise, snapping or window focus. Sheets can't move at all, and the stacking order is probably lost on restore (ui-layout.toml's table sorts ids).

## Proposal

- New fields on `ui.window`: `kind` (app | panel | dialog | overlay), `min = {w, h}`, `esc` (closes on Esc), `dock` and `launcher` (listed in them).
- Engine state per window: minimised, maximised and snapped (with the size to restore), and the focused app.
- Snapping to the free area between the docked columns: halves, the top to maximise, corners to quarters.
- Double-click the title to maximise.
- Ordered stacking saved as a list.
- Minimised windows aren't built.
- Only the focused app rebuilds at the fast rate; others go to slow.
- At most six apps open; the least recently used minimises.

## Acceptance criteria

- [ ] rim_ui tests: minimise and restore keep state; snapping gives the named rectangles
- [ ] A minimised window costs no rebuilds (ui.builds over 60 frames)
- [ ] docs/modding/ui.md documents the new fields; api_types regenerated
- [ ] The steady-HUD test still passes: apps never move a docked panel
