---
id: 8d551753-0e15-4e00-8930-b1bb62efd2dc
title: 'Hauling: colonists carry loose items to stockpiles'
type: feature
status: backlog
milestone: colony
depends_on:
- ecd54de8-e9f2-4767-a746-d751f906fb40
created: 2026-09-25
updated: 2026-09-25
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

- [ ] A loose item is carried into a stockpile that allows it (test)
- [ ] A disallowed item is carried out to one that allows it
- [ ] Haul at priority 0 is never chosen; Build 1 Haul 2 builds first (test)
- [ ] Determinism test passes with zones and hauling
