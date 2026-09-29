---
id: 373
uid: 827b2421-129a-4995-b475-1398dc91d2cb
title: 'House plans as data: an ASCII grid placed with one command'
type: feature
status: done
milestone: houses
assignee: Oddur Sigurdsson
depends_on:
- 398
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
priority: p2
api: additive
effort: m
layer: engine
area: building
---

## Why

Minecraft structures and Factorio blueprints make a good house something you place again. It's text, with no binaries, and the balance bots could build from plans instead of hand-coded huts. DESIGN.md §6c.

## What

- A `[[plan]]` def: `grid` (text), `legend` (character to thing), and an optional material per character.
- `Command::PlacePlan { plan, at, facing, stuff }` places every piece as a blueprint, skipping cells where it can't.
- Primitive ships a branch hut and a cob house. The stone_age bot uses them.
- The client saves a selection as a plan file, in the same format.

## Acceptance criteria

- [x] A placed plan turned east matches the grid turned east (test)
- [x] The stone_age sweep builds its hut from a plan and holds its numbers

## 2026-09-27

Split the client halves out: placing a plan from the build menu (b6a2d3cf) and saving a selection as a plan (c281689c). This item ships the def, the command and primitive's two plans. The stone_age bot builds from primitive:branch_hut (--plan ID picks another). Its door moved from the corner of the old hand-coded 3x3 to the middle of the south wall, where a door belongs. 20 seeds, 5 days: every target holds (first night 100%, flint 100%, felled 100%, cob walls 90%).
