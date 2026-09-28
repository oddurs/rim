---
id: 3fea3b3d-9c8b-4614-9e51-9f43d5576025
title: 'Movement classes: a creature that drops a level, with its own region layer'
type: feature
status: backlog
milestone: depth
depends_on:
- ba8253df-9f73-4196-87ac-2924e3143627
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: additive
effort: m
layer: engine
area: pathing
---

## Why

Split from ba8253df (pits and bridges): trenches and bridges stand on their own, and a second region layer per movement class is an engine change of its own size (DESIGN.md §6d).

## What

- A `[[movement]]` def (`drop = 1`) whose region layer treats a one-level drop as passable. Region layers go from per faction to per faction and class.
- Core ships no climbers; the class exists for mods.

## Acceptance criteria

- [ ] A fixture mod's climber crosses a one-level drop; the extra flood fill's cost is recorded here
- [ ] Determinism test passes
