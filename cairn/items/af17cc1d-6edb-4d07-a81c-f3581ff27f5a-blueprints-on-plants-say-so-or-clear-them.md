---
id: af17cc1d-6edb-4d07-a81c-f3581ff27f5a
title: 'Blueprints on plants: say so, or clear them'
type: bug
status: doing
milestone: stone-age
assignee: Oddur Sigurdsson
claimed: 2026-09-25
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

- [x] A wall dragged across tall grass has no silent gaps
- [x] The player sees what's in the way, or it's cleared as part of the job

## 2026-09-25

Cleared as part of the job, rather than reported. A building planned over a natural thing marks it with the harvest that removes it (gather for grass, chop for an oak, mine for rock) and puts a Planned on it. When it's despawned, the blueprint goes up in its place. With no clearing harvest (a berry bush regrows), it's cleared at once. Cancel takes the plan off and leaves the thing. Floors lie under plants and are unaffected. place_lot no longer drops items on a blueprint's cell, so the grass's fibre isn't buried under the wall it made way for. Saved as an optional engine:planned section, as engine:order was.

## 2026-09-25

Review fixes:
- A planned thing keeps its clearing mark: designating gather over a planned oak no longer swaps chop for a harvest that leaves the oak, which stalled the wall forever.
- place_lot also skips cells planned for a building, not only blueprints, so a neighbour's yield isn't buried when its turn comes.
- Something that blocks and has no clearing harvest is left, not removed for free.
- Only ground that can be built on is planned over.
- A plan whose building or material is gone on load is dropped, rather than clearing the thing for a blueprint that can't be placed.
Build over loose items, which predates this, is filed separately.
