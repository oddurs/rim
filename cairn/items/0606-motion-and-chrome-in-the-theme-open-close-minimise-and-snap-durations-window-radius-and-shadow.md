---
id: 606
uid: 432dfc0d-5f3a-467b-b13b-61fabcb07ddd
title: 'Motion and chrome in the theme: open, close, minimise and snap durations, window radius and shadow'
type: feature
status: backlog
milestone: rimos
depends_on:
- 586
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: additive
effort: s
layer: client
area: ui
---

## Problem

The 0.14 s open and the window shadow are hard-coded in rim_ui's lib.rs, and reduce motion has to be special-cased wherever something moves.

## Proposal

- New theme sections: `[motion]` (open 0.14, close 0.10, minimise 0.18, snap 0.12, menu 0.09, toast 0.16) and `[shape] radius_window = 6`, plus a shadow colour and size.
- Reduce motion sets every duration to 0.
- Animation is an offset and alpha applied at paint to the built mesh, never a rebuild.

## Acceptance criteria

- [ ] With reduce_motion on, a window opens fully drawn on the first frame (test)
- [ ] Opening, moving and minimising cause no rebuild of the window's tree (ui.builds)
- [ ] docs/modding/ui.md lists the tokens
