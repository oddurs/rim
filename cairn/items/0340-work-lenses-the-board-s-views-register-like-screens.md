---
id: 340
uid: 4c9fc19b-3b7b-4bc3-93b4-7b5d600cbc8d
title: 'Work lenses: the board''s views register like screens'
type: feature
status: done
milestone: work
assignee: Oddur Sigurdsson
depends_on:
- 309
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: additive
effort: s
layer: core
area: modding
---

## Why

The Work screen will carry the board, the Roles lens and one person as shelves, and DESIGN.md §4d wants mods to add their own (a headcount view, a labour list). Today `ui/work.luau` hard-codes one window and a separate ranked window.

## What

- `require("@core/ui/work").lens({ id, label, order, draw = fn(view, board) })`, the same shape as `screens.add`. A second `lens` with the same id replaces the first, so a reloaded script doesn't double up.
- The Work window draws a tab per lens in `order`, remembers the last one, and `Tab` moves to the next.
- Board and Person (today's ranked view as shelves) move onto the registry. Lenses only write through `act.*`, so no lens can disagree with another.
- A fixture mod in the UI tests adds a lens that lists work types by waiting count.

## Acceptance criteria

- [x] A fixture mod's lens appears as a tab, and its writes show on the board (UI test)
- [x] Board and Person are lenses; the separate ranked window is gone
- [x] `docs/modding/api-ui.md` documents `lens`

## 2026-09-26

Dropped the Tab key for the next lens: with nothing focused, Tab belongs to the game (next colonist), and taking it inside the Work window would make Tab mean two things. Tabs are clickable, and a binding can come later if players ask.
