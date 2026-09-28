---
id: 1104bf12-b1af-4387-a74a-2e1e9661a9d0
title: 'Light crosses openings: a level lit through its stairwells and air by the levels beside it'
type: feature
status: backlog
milestone: lighting
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
effort: m
layer: client
area: render
depends_on:
- 3124bd7b-9f9a-4d92-84e6-2df736b6e2fe
---

## Why

A torch at the top of a stairwell should light the steps below, and a fire in a pit should glow on its rim. With each level lit on its own (3124bd7b), light stops dead at every opening. DESIGN.md §6e, Depth.

## What

- Each level's light adds the level above's through its openings (stairwells, ladders) and the level below's through its air cells, read from a blurred half-resolution copy of that level's light and attenuated 0.5 per level.
- Skipped on a level with no openings.

## Acceptance criteria

- [ ] A torch at the top of a stairwell lights the steps on the level below, fading with distance from the opening, with no visible seam at the opening's edge (screenshot pair, both levels)
