---
id: 471
uid: 463983bb-9bd0-4a31-bfae-f7f814e9b01d
title: 'Box select: a snapped box that previews, Shift adds, Alt removes'
type: feature
status: done
milestone: chalkline
assignee: Oddur Sigurdsson
depends_on:
- 482
- 516
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: none
effort: s
layer: client
area: ui
---

## Why

A select drag draws a yellow-ish box in the tool colour with no idea of what it will take. There is also no way to drop colonists from a group except by shift-clicking them one at a time. DESIGN.md §6f.

## What

- The box is snapped to cells: a 1 px chalk edge on a keyline, with a 5% chalk fill. While it is up, the grid is at `Lens`.
- Colonists inside show their hover ring before release.
- Chip: "5 × 4 · 3 colonists". With Shift held it reads "+2" (only newcomers preview). With Alt held the edge is dashed, the chip reads "−1", and colonists that would leave drop to 30%.
- Alt-drag removes the boxed colonists from the selection. This is the only change to what box select does.

## Acceptance criteria

- [x] Autotest: a box over two colonists previews two hover rings and the chip reads "… · 2 colonists"
- [x] Autotest: with three colonists selected, an Alt-drag over one leaves two selected
- [x] Autotest: a Shift-drag adds without dropping anyone
- [x] Screenshot `chalk-box`

## 2026-09-27

This item adds the theme token hair (and its doc row) to mods/core/ui/theme.toml and docs/modding/ui.md: tokens land with the item that first reads them (d83192ed review).

## 2026-09-27

Stacked on zones (11080b20) → hover → grid → palette; rebases as the stack merges.

## 2026-09-27

Review: the preview and the release share one test for a box (is_box: moved 6 points from the press), so a preview never shows what the release won't do. Ctrl or Cmd take colonists out as well as Alt, since many Linux desktops keep Alt-drag for moving windows. An Alt-drag that takes no one changes nothing. A Shift-box joins colonists only. The drag's count goes into the UI's pointer hint, so there's one chip by the pointer.

## 2026-09-28

Criterion 1 as tested: the box is over three colonists, not two. It checks one ring per boxed colonist (the preview uses selection's ring, as the release will pick them) and the pointer hint '5 × 5 · 3 colonists'. Same claim, different count.
