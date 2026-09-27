---
id: 463983bb-9bd0-4a31-bfae-f7f814e9b01d
title: 'Box select: a snapped box that previews, Shift adds, Alt removes'
type: feature
status: planned
milestone: chalkline
depends_on:
- 553bfb19-5924-4d0a-86f7-a726b6c70d6c
- bd7a158e-394b-438e-a93c-555c1f80cf59
created: 2026-09-27
updated: 2026-09-27
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

- [ ] Autotest: a box over two colonists previews two hover rings and the chip reads "… · 2 colonists"
- [ ] Autotest: with three colonists selected, an Alt-drag over one leaves two selected
- [ ] Autotest: a Shift-drag adds without dropping anyone
- [ ] Screenshot `chalk-box`

## 2026-09-27

This item adds the theme token hair (and its doc row) to mods/core/ui/theme.toml and docs/modding/ui.md: tokens land with the item that first reads them (d83192ed review).
