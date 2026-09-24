---
id: 214
title: Core collapses to one wall, one door, one bed
type: content
status: done
milestone: building
assignee: Oddur Sigurdsson
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

- [x] No `<thing>_<material>` defs left in core
- [x] `mods/marble` -- one item def, nothing else -- builds every buildable
- [x] The Castaway opening still plays: chop, haul, four walls by nightfall

## 2026-09-23

Salvaged metal dropped from this item: nothing in core, wildlife_plus or weather produces metal -- no ore, no salvage, no drop -- so it would be a material that can never appear in a game. Wood and stone exercise the system and the marble test proves a third material is one item def. If a source arrives (raider gear from 0103, mining ore), the material is one def in whichever mod adds the source.

## 2026-09-23

Done. Door and bed take 10 and 25 of anything structural; a bed of stone is odd but the sprint promised every buildable in marble, and a 'fine' category for furniture is one line whenever content wants it. The marble test now enumerates every buildable with stuff and builds each, so a future buildable is covered without editing the test. Criterion 3 rests on gameplay.rs (chop, haul, walls up, headless) plus the Linux autotest in CI clicking core:toolbar.build:wall through default_material. Metal: see the earlier note -- no source, so no def.
