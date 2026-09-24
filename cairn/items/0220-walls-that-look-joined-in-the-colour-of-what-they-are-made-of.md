---
id: 220
title: Walls that look joined, in the colour of what they are made of
type: feature
status: doing
milestone: building
assignee: Oddur Sigurdsson
claimed: 2026-09-24
depends_on:
- 214
created: 2026-09-23
updated: 2026-09-24
priority: p1
api: none
effort: m
layer: client
area: render
---

## Why

One `wall` def in six materials has to read as six different walls, and a
run of wall has to read as a wall rather than a row of blocks.

## What

- Autotiling: a wall's drawn shape comes from its blocking neighbours, so
  corners, tees and ends join.
- Tint from the material, not the def, so marble arrives looking like
  marble without a renderer change.
- Doors and windows read as openings in the run, not separate objects sat
  next to it.

## Acceptance criteria

- [ ] A stone wall and a wooden wall are told apart at a glance
- [ ] Corners and junctions join with no gaps
- [ ] A material added by a mod is drawn correctly with no client change
