---
id: 3ab05834-fbf6-4876-a411-bfce0c18523a
title: 'Floors: built ground that remembers what it is made of'
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
created: 2026-09-24
updated: 2026-09-24
priority: p1
api: additive
effort: m
layer: engine
area: building
---

## Why

A room is walls and what is inside them; the ground it stands on is still
grass. Floors are the first thing built *on* a cell rather than *in* it,
and they have to sit under beds, tables and walls without fighting them
for the cell.

## What

- `category = "floor"` on a thing def puts it in a new map layer under
  fixtures and items. One cell can hold a floor and a bed. A floor's
  `path_cost` replaces the terrain's while it is there, so a floor over
  mud is a fast floor.
- Floors are things like everything else: built of whatever is structural,
  owned, `MadeOf`, drawn in the material's colour, deconstructed for a
  refund. Nothing about them bounds rooms or blocks fields.
- Core gets one `floor` def. A mod that wants tiles or carpet adds a def.

## Acceptance criteria

- [x] A floor and a bed can share a cell; a floor and a wall too
- [x] Walking over a floor costs what the floor says, not what the ground did
- [x] Rooms and boundaries are unchanged by floors
- [x] Deconstruct refunds a floor in the material it was made of
- [x] Drawn under everything, tinted by material

## 2026-09-24

Done. A floor is a thing with category = floor: it lives in a third map layer under items and fixtures, so a floor and a bed, or a floor and a wall, share a cell. Its path_cost stands in for the terrain's while it is there (a blueprint changes nothing); passability never changes, so no region or room rebuild, and floors are never boundary pieces -- a test walls a hut, floors the inside in stone, and asserts cells, boundary count, leak multiplier and reachability are unchanged. Everything else is reuse: stuff, MadeOf, Owner, factors, tint from the material, deconstruct (the Built arm and Cancel now scan the floor layer), right-click. Criterion 5 (drawn under everything, tinted) is implemented -- layer 0 in the draw loop, Shape::Floor, material colour via MadeOf -- but not pixel-proven; the autotest builds no floor. The marble test enumerated the new floor on its own and failed because its cell lookup knew only the fixture layer: the fourth call site to learn about floors, and the one that proves the test is doing its job.
