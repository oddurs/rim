---
id: bb769d00-9370-42e9-90a1-00adb1add366
title: 'Designate: targets light in their hue, on hover and across a drag'
type: feature
status: planned
milestone: chalkline
depends_on:
- cd59b515-b98c-4f24-92f8-ec55161d22aa
- d83192ed-0a3e-4a6f-a39a-04ce94a68935
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: none
effort: m
layer: client
area: ui
---

## Why

A chop drag fills its whole rectangle green at 18% (`draw.rs` world_ui). It doesn't show which trees it will mark, and it doesn't show that half of them are already marked. DESIGN.md §6f.

## What

- **Hover**, with a designate tool:
  - If `designate_preview` for the one cell names a target, the target gets an edge in the designation's hue and a chip such as "Chop · tree".
  - An already-marked target shows "Already marked".
  - Anything else gets a faint 1 px chalk frame at 38%, with no chip.
- **Drag:**
  - The box is a 1 px edge in the hue with a 6% fill.
  - Each new target gets a 1.5 px ring in the hue and a preview dot at 60%. Marks already there are left alone.
  - A chip counts the new targets: "Chop · 4 trees".
- **Cancel tool:** the box is dashed chalk, and each mark or plan it would clear drops to 30%, with the chip "Cancel · N".
- **Refused click:** a click that marks nothing shakes the frame twice by 3 px over 180 ms and shows the reason for 1.4 s.

## Acceptance criteria

- [ ] Autotest: a chop drag over five trees, two already marked, gives exactly three ring marks and a chip reading "Chop · 3 trees"
- [ ] Autotest: hovering bare grass with chop gives a faint frame and no chip
- [ ] After release, the trees marked are the ones that were ringed (autotest)
- [ ] The whole-rectangle fill for designate tools is gone from `world_ui`
- [ ] Screenshot `chalk-designate`
