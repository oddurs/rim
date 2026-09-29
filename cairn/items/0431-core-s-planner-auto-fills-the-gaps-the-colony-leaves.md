---
id: 431
uid: f5295002-f21b-4d73-a37a-308f49dc2487
title: 'Core''s planner: Auto fills the gaps the colony leaves'
type: feature
status: done
milestone: work
assignee: Oddur Sigurdsson
depends_on:
- 413
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p0
api: none
effort: m
layer: core
area: ai
---

## Why

The mechanism does nothing without a policy. DESIGN.md §4d gives core's: need, gap, fill, rest, planned hourly around role members and pins.

## What

- `mods/core/scripts/auto.luau` registers `rim.planner("core:auto", …)`:
  1. Need: `ceil(waiting / auto.per_person)`, at most the colony's size, 0 when nothing waits.
  2. Gap: minus colonists at First or Soon whom the planner doesn't move.
  3. Fill: first person per gap before any second; larger `waiting × weight` first; best skill wins, less 25 per First already given; at most two First each, then Soon.
  4. Rest: Later if skilled (5+) or skill-free, Spare time if unskilled. Never 0.
- Reasons in plain words: "fills Haul: 23 waiting, wants 3, has 2", "covered; plants 9", "nothing waiting".
- `auto` values on core's work types: build `per_person = 4, weight = 4`; mine 6, 1; chop 4, 2; harvest 5, 3; hunt 1, 2; haul 8, 1. The crafting mod's craft 2, 2.
- Luau tests in `mods/core/tests` for the planner as a pure function over a board table.
- A sim test: a castaway on Auto with a shelter blueprint and berries waiting builds and forages in its first day; six colonists with 23 loose items put someone on Haul within two in-game hours.

## Acceptance criteria

- [x] The planner never proposes 0 and never touches a pin (Luau test)
- [x] With every gap covered by role members, Auto colonists get no First for those work types (Luau test)
- [x] The castaway and six-colonist sim tests pass
- [x] One run costs under 0.5 ms at 30 colonists × 12 work types, recorded in DESIGN.md §4d
