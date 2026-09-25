---
id: 8d551753-0e15-4e00-8930-b1bb62efd2dc
title: 'Hauling: colonists carry loose items to stockpiles'
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
depends_on:
- ecd54de8-e9f2-4767-a746-d751f906fb40
created: 2026-09-25
updated: 2026-09-25
closed_at: 2026-09-25
priority: p0
api: additive
effort: m
layer: engine
area: ai
pillar:
- growth
---

## Why

Items should end up somewhere useful, not where they fell. Split from
ecd54de8, which makes the zones; this is the work that fills them.

## What

- A core `haul` work type claiming a new engine job, "haul" (DESIGN.md §4d).
- A loose item, or one in a zone that doesn't allow it, is haul work if
  some stockpile allows it and has room: an empty cell, or a stack of the
  same item below its limit.
- `Job::Haul`: pick up (a carry's worth), walk, set down on the chosen
  cell, merging into the stack there; if the cell has filled meanwhile, the
  next free one in the zone, else drop nearby.
- Items in a zone that allows them are never hauled again.

## Acceptance criteria

- [x] A loose item is carried into a stockpile that allows it (test)
- [x] A disallowed item is carried out to one that allows it
- [x] Haul at priority 0 is never chosen; Build 1 Haul 2 builds first (test)
- [x] Determinism test passes with zones and hauling

## 2026-09-25

A haul engine job, claimed by core's new haul work type (order 60, after hunting). find_haul: the nearest loose stack (on the item layer, not where a zone keeps it) that some zone takes, and the nearest cell in such a zone with room (World::room_for: empty, or the same item below its stack limit), skipping cells another hauler is bound for; ranked by the walk to the stack plus on to the cell. run_haul takes what the cell has room for (up to a carry), walks, and World::put_item sets it on exactly that cell; anything left rides out in the carry and end_job drops it nearby, as for delivery. Scans every zone cell per candidate (pools replace this).

## 2026-09-25

Review: room_for ignored blueprints, so hauling filled a planned wall's cell and the wall went up over the stack, lost for good; a cell with any fixture now has no room (test fails without it). An idle hauler with every stockpile full rescanned everything each think (0.87-0.94 ms a step on 250x250, 30 colonists, a 40x40 stockpile, 300 loose stacks): now haul is skipped when a better-level job was found, whether any zone has room is decided once per item def, and zones keep their own cell lists (Zones::members) so a search walks a zone, not the map: 0.33 ms a step on the same setup, 0.21 with room. The rest is walking every Thing, which the work pools item removes. A haul result also no longer overwrites a same-type designation job it doesn't beat.
