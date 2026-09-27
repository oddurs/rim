---
id: bd7a158e-394b-438e-a93c-555c1f80cf59
title: 'Hover: an edge on the thing under the pointer'
type: feature
status: planned
milestone: chalkline
depends_on:
- d83192ed-0a3e-4a6f-a39a-04ce94a68935
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: none
effort: s
layer: client
area: ui
---

## Why

Nothing on the map shows what a click would pick. The hover readout names it in a corner of the screen, but the eye is on the map. DESIGN.md §6f, "out, edge, in".

## What

With the select tool:
- A thing under the pointer (`thing_under`) gets a rounded edge on its footprint: 1.5 px chalk at 72%, radius up to 5 px, on a keyline.
- A pawn under the pointer (`pawn_under`) gets a 1.25 px ring 3 px out from its body.
- A stockpile cell brightens its zone's perimeter to 1.5 px.
- Empty ground draws nothing, and nothing is drawn while the pointer is over the UI.
- The edge fades in over 60 ms and out over 140 ms, and a thing that is both hovered and selected shows both.

## Acceptance criteria

- [ ] Autotest: hovering a tree gives one hover mark on its cell; hovering bare grass gives none; hovering a panel gives none
- [ ] Autotest: a selected, hovered colonist shows both a hover ring and a selection ring, and their radii differ by at least 2 px
- [ ] Screenshot `chalk-hover`

## 2026-09-27

This item adds the theme token stroke (and its doc row) to mods/core/ui/theme.toml and docs/modding/ui.md: tokens land with the item that first reads them (d83192ed review).
