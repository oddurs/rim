---
id: 26a69a8e-a7e1-44ab-a05c-96dda82d4074
title: 'Pipe joins: fences and palisades connect like Minecraft''s'
type: feature
status: backlog
milestone: houses
depends_on:
- 3fe8c3cb-dcba-4882-b623-0468ea9fe697
created: 2026-09-26
updated: 2026-09-26
priority: p2
api: additive
effort: s
layer: client
area: render
---

## Why

A fence is a thin barrier, and it should look it: a post, with rails toward each neighbour. It should meet a wall without the wall bulging toward it. DESIGN.md §6c.

## What

- `style = "pipe"`: a post in the middle of the cell, and a pair of rails toward each orthogonally joined neighbour.
- `connects = ["wall"]` lets a fence join a wall's side without changing the wall's mask.
- Primitive gets a branch fence (a hurdle) and core a log fence, so pens have something to be built of.

## Acceptance criteria

- [ ] A fence meeting a wall draws its rail into the wall, and the wall's contour doesn't change
- [ ] A fence gate is a door with a pipe look
