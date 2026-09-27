---
id: b6a2d3cf-e7aa-4d89-a316-3f78351c9926
title: Place a house plan from the build menu, turned with T
type: feature
status: backlog
milestone: houses
depends_on:
- 827b2421-129a-4995-b475-1398dc91d2cb
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: none
effort: m
layer: client
area: ui
---

## Why

`Command::PlacePlan` (827b2421) places a house plan whole, but a player can only reach it from a script. The build menu should offer plans as it offers walls.

## What

- Each `[[plan]]` is a tool in the build tray, in a "plans" group.
- The cursor carries the plan's cells as a ghost, turned with T as a single build is. A click places it with the material picked in the tray.

## Acceptance criteria

- [ ] The autotest places primitive's branch hut from the tray, turned once, and every piece is planned where the ghost showed it
