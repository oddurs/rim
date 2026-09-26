---
id: f709cdd4-fdb0-4de1-b0d9-4311e7fcd928
title: Room roles as data
type: feature
status: doing
milestone: houses
assignee: Oddur Sigurdsson
claimed: 2026-09-26
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: m
layer: engine
area: sim
---

## Why

Eras ("Camp: a shelter, a bed and a fire"), mood ("slept in a barracks") and the storyteller all need to know what a room is for. If each works it out in its own script, they will disagree. DESIGN.md §6c.

## What

- A `[[room_role]]` def: `needs` (tag counts) and optional `min_cells` and `indoor`. The first role a room meets, in load order, names it.
- The engine counts the tags of the things inside each room at room rebuild and when a thing inside is built or removed.
- Core declares `dormitory`, `home`, `bedroom` and `hall`. Crafting declares `workshop`.
- `rim.room_at(x, y)` gains `role`, `cells`, `leak`, `daylight` and `heated`, and `room_changed` fires when a role changes.

## Acceptance criteria

- [x] A room with a bed and a fire is a `home`; add a second bed and it becomes a `dormitory` (test)
- [x] A mod adds a role by data alone and a script reads it
- [x] No per-tick cost: role work runs only at room rebuild and on building changes

## 2026-09-26

Deferred the room_changed event: room ids renumber on every rebuild, so 'this room's role changed' has no stable meaning yet, and nothing reads it. Mood can add it with a stable room identity when it needs one. Roles count things in the room of their anchor; a blocking thing (a workbench) counts for the first room beside it.
