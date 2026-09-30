---
id: 698
uid: f0e576e3-318f-4062-877f-5debfc268819
title: Script reads rebuild rooms mid-tick and misread room temperatures
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p2
api: none
layer: engine
area: scripting
---

## What

`rim.field`, `rim.indoors` and `rim.room_at` (script.rs ~1182, ~1459, ~1469) call `w.map.ensure_rooms()` before reading. They are the only mid-tick callers. `World::has_shelter`'s comment says why the engine doesn't: rebuilding rooms mid-tick renumbers them under the fields' room values.
- `value_fixed` reads `layer.rooms[r.id - 1]` with the new id against values numbered by the old rooms, so a script reads another room's temperature.
- `Map` keeps one previous grid (`prev_room`). Two rebuilds between two `fields.update` calls make `carry_rooms_over` carry from the wrong grid, so room values end up scrambled.

## How it fails

The shipped fire mod reads `rim.field` for each burning cell and calls `rim.damage` on walls. Two wooden walls burning through in one step means two rebuilds in one tick. The result is deterministic (not a desync), but room temperatures come out wrong.

## Reproduce

Not yet run; verified by reading. A hut with two rooms at different temperatures, and a probe hook: `rim.field`, remove wall 1, `rim.field`, remove wall 2, `rim.field`. Compare room values after one step with the same removals made without the reads.

## Fix

Script reads see rooms as built at the start of the step, as `has_shelter` does: drop `ensure_rooms` from these bindings. This changes sim behaviour (full).

## Acceptance

- [x] A script read never rebuilds rooms mid-tick
- [x] A test that fails before the fix and passes after
