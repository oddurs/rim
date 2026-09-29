---
id: 309
uid: 166a4cc9-fee8-4cd4-9384-4ec243f74031
title: 'Named levels and pins: First, Soon, Later, Spare time'
type: feature
status: done
milestone: work
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p0
api: additive
effort: s
layer: core
area: ui
---

## Why

The board speaks in numbers a new player can't read, and it can't say whether a value is the player's or a default. DESIGN.md §4d: levels have names, and a cell says who set it. A setting equal to the inherited value is noise: it pins a colonist to today's default when the default later changes.

## What

- `labels` on `[[priority_scale]]`, one per level, validated to match `levels`. Core: `["First", "Soon", "Later", "Spare time"]`. A scale without labels shows numbers, so a nine-level mod keeps working.
- `view.board()` returns the labels and, per cell, whether it is pinned (the colonist has their own setting) or inherited.
- `Command::SetPriority` takes `level | inherit`. `inherit` removes the colonist's setting; the pawn's list shrinks, so the save and hash forget it.
- The board: a pinned cell is bold with a dot; an inherited one plain. A click that lands on the inherited value sends `inherit`. `A` while hovering hands a cell back. Tooltips and the ranked view use the names.
- The ranked view becomes shelves: one row per level, work types as tokens, click to move.

## Acceptance criteria

- [x] A scale whose `labels` length differs from `levels` fails to load with a message naming the mod (test)
- [x] `SetPriority` with `inherit` removes the setting, and the state hash equals a pawn that never had one (test)
- [x] Cycling a cell back to its default leaves no pin on the board (autotest or UI test)
- [x] The board and ranked view show names; a nine-level fixture scale shows numbers
- [x] `scripts/check-luau.sh` and the determinism test pass

## 2026-09-26

SetPriority's field type stays u8: saves log commands, so changing it to level | inherit would break old logs. Handing back is a new ClearPriority command instead. The label count must match levels or the load fails, so a mod growing the scale patches levels = 9, labels = [] (DESIGN.md updated to say so).
