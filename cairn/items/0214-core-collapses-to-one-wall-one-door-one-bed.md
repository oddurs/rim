---
id: 214
title: Core collapses to one wall, one door, one bed
type: content
status: backlog
milestone: building
depends_on:
- 212
- 213
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: none
effort: s
layer: core
area: building
---

## Why

The material system is only real once the duplicate defs are gone.

## What

- `wall_wood` and `wall_stone` become one `wall`. Same for `door_wood` and
  `bed_wood`.
- `wood`, `stone` and a salvaged metal declare their `stuff` categories and
  factors. Three materials: enough to prove the system without inventing a
  resource economy.
- The toolbar shows one button per buildable, not one per material.

## Acceptance criteria

- [ ] No `<thing>_<material>` defs left in core
- [ ] `mods/marble` -- one item def, nothing else -- builds every buildable
- [ ] The Castaway opening still plays: chop, haul, four walls by nightfall
