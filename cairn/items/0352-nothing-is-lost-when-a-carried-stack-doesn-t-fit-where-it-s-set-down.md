---
id: 352
uid: 629e1fa7-7e3c-4c42-96dd-98ea8a3b762f
title: Nothing is lost when a carried stack doesn't fit where it's set down
type: bug
status: done
milestone: crafting
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: none
effort: s
layer: engine
area: ai
---

## Why

`end_job` drops a carry with `place_lot`, which spirals out to radius 8 and returns what didn't fit; `ai.rs:74` ignores it, so the rest of the stack disappears. A hauler whose cell filled while it walked can lose items the same way. DESIGN.md §4f: nothing is lost.

## What

- Whatever `place_lot` couldn't fit is placed further out, or stays carried until it can be, and is never dropped on the floor of nowhere.
- A test that fails without the fix: a crowded drop with more than the radius holds.

## Acceptance criteria

- [x] A test that fails before the fix: a carried stack dropped where radius 8 is full keeps every item
- [x] Every caller of `place_lot` handles what's left

## 2026-09-26

place_lot now walks rings outward until everything is down (up to the map's size), iterating only each ring's cells in the same row order as before, so placements within radius 8 are unchanged and a far drop costs O(ring) per ring rather than O(square). It returns a remainder only when the whole map is full, so callers that ignore it lose nothing in practice. Noticed: place_lot will drop onto a passable fixture's cell (under a tree, onto a bed), which room_for refuses; left as it was, since hauling reads room_for and nothing is lost.

## 2026-09-26

put_down had the same loss: past radius 8 it despawned a held tool. It now walks rings to the map's size as place_lot does. Test a_tool_put_down_in_a_crowded_place_is_kept fails without it.
