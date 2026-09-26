---
id: 629e1fa7-7e3c-4c42-96dd-98ea8a3b762f
title: Nothing is lost when a carried stack doesn't fit where it's set down
type: bug
status: backlog
milestone: crafting
created: 2026-09-26
updated: 2026-09-26
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

- [ ] A test that fails before the fix: a carried stack dropped where radius 8 is full keeps every item
- [ ] Every caller of `place_lot` handles what's left
