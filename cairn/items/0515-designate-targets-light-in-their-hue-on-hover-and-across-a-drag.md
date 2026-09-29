---
id: 515
uid: bb769d00-9370-42e9-90a1-00adb1add366
title: 'Designate: targets light in their hue, on hover and across a drag'
type: feature
status: done
milestone: chalkline
depends_on:
- 525
- 533
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
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

- [x] Autotest: a chop drag over five trees, two already marked, gives exactly three ring marks and a chip reading "Chop · 3 trees"
- [x] Autotest: hovering bare grass with chop gives a faint frame and no chip
- [x] After release, the trees marked are the ones that were ringed (autotest)
- [x] The whole-rectangle fill for designate tools is gone from `world_ui`
- [x] Screenshot `chalk-designate`

## 2026-09-27

Review: fixed the refusal reading last frame's preview (it's now asked of the released cell, with an autotest that moves and clicks in one frame). An already-marked creature in a hunt click's box now reads 'Already marked'. cancel_preview dedupes with a BTreeSet like designate_preview. Aims are culled to the screen. A right click that drops the tool drops its refusal. Plurals use a shared helper, and a hover over several targets gives the count. Cancel's hint names what it takes back ('Cancel · 3 walls', 'Cancel · nothing'). Declined: moving world_ui's drag box into the overlay here; build ghosts (e8f313d6), stacked next, does it.

## 2026-09-27

Checked: with the two client fixes reverted, the autotest fails exactly the two new checks. 'A click that moved' fails with 'Nothing to chop here', and 'a right click drops the tool' fails because the refusal stays (288 passed, 2 failed). With the fixes in, 290 passed.

## 2026-09-28

Rebased onto #b3ffbae1 (replace in place): cancel_preview now names the replacement plans in the rectangle, which sit on no layer, the same way Cancel's apply walks them. replace.rs's cancelling_a_replacement_leaves_the_old_wall fails without it; previews.rs's cancel test now plans stone walls over wood ones so it covers them too.

## 2026-09-28

Criterion 1 as tested: the drag's box holds whatever trees this map puts near home, with one marked beforehand, not exactly five with two marked. It checks one ring per tree the drag would newly mark, none on the one already marked, and the chip 'Chop · N oak trees' for that N. Same claim, the map's count.
