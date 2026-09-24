---
id: 216
title: 'Rooms made of something: leak and daylight from the boundary'
type: feature
status: backlog
milestone: building
depends_on:
- 211
- 213
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: additive
effort: l
layer: engine
area: map
---

## Why

A room ringed in stone should hold heat differently from the same room in
wood. Until it does, materials are a number in a tooltip.

## What

Depends on the answer from 0211. Assuming the boundary option:

- `[[thing]]` gains boundary contributions (`leak`, `daylight`) -- what
  this piece of wall does to the room it helps enclose.
- A room's leak and daylight are summed over its boundary cells, scaled by
  each piece's material factors, and cached. Rooms already rebuild only
  when walls change, so this rides that.
- Field defs keep their constant as the default for a boundary that says
  nothing.

## Acceptance criteria

- [ ] A stone room and a wooden room of the same shape reach different
      temperatures from the same fire
- [ ] Visible in the temperature overlay
- [ ] Boundary recompute stays inside the room-rebuild budget
