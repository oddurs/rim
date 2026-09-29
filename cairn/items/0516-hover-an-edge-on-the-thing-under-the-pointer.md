---
id: 516
uid: bd7a158e-394b-438e-a93c-555c1f80cf59
title: 'Hover: an edge on the thing under the pointer'
type: feature
status: done
milestone: chalkline
assignee: Oddur Sigurdsson
depends_on:
- 533
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
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
- A pawn under the pointer (`pawn_under`) gets a `stroke` ring on the edge of its body, as a thing's hover lies on its footprint's edge. Its selection ring sits `bracket_gap` outside the body, so the two never touch.
- A stockpile cell brightens its zone's perimeter to 1.5 px.
- Empty ground draws nothing, and nothing is drawn while the pointer is over the UI.
- The edge fades in over 60 ms and out over 140 ms, and a thing that is both hovered and selected shows both.

## Acceptance criteria

- [x] Autotest: hovering a tree gives one hover mark on its cell; hovering bare grass gives none; hovering a panel gives none
- [x] Autotest: a selected, hovered colonist shows both a hover ring and a selection ring, and their radii differ by at least 2 px
- [x] Screenshot `chalk-hover`

## 2026-09-27

This item adds the theme token stroke (and its doc row) to mods/core/ui/theme.toml and docs/modding/ui.md: tokens land with the item that first reads them (d83192ed review).

## 2026-09-27

Started stacked on the grid branch (553bfb19), for app.pointer; rebases onto main as the stack merges.

## 2026-09-27

Review: hover fades are per target now (each fades from wherever it had got to), so crossing a gap or moving between stockpiles no longer drops a fade. Ring radii come from one function (overlay::ring_radius) that both the renderer and the autotest read. The pawn hover ring moved onto the body's edge to mirror things; the spec's '3 px out' would have overlapped selection's keyline.
