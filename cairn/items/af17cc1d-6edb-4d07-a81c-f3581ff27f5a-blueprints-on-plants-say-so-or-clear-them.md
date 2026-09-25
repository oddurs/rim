---
id: af17cc1d-6edb-4d07-a81c-f3581ff27f5a
title: 'Blueprints on plants: say so, or clear them'
type: bug
status: backlog
milestone: stone-age
created: 2026-09-25
updated: 2026-09-25
priority: p1
api: none
effort: s
layer: engine
area: building
---

## Why

`spawn_fixture_of` (`crates/rim_sim/src/world.rs`) refuses a cell that already holds a fixture, and `Command::Build` ignores the refusal. Drag a wall across a patch of grass, a bush or a tree, and those cells are silently left out: the wall has gaps, no room forms, and the colonist freezes on the night the wall was for. The stone age adds wild things to grass and dirt (mods/primitive), so it happens more.

## What

- A blueprint over a natural plant or rock is placed anyway, and the thing under it is marked to be cleared first: harvested with its own designation, or simply removed if it has none.
- Or, at the least, the refused cells are reported to the player ("3 cells blocked by tall grass").

## Acceptance criteria

- [ ] A wall dragged across tall grass has no silent gaps
- [ ] The player sees what's in the way, or it's cleared as part of the job
