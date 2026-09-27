---
id: c281689c-26c5-4352-9299-4cda3d307bd0
title: Save a selection as a house plan
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
area: building
---

## Why

Minecraft structures and Factorio blueprints make a good house something you place again (DESIGN.md §6c). A plan the player built should be savable in the same text format mods ship.

## What

- Drag a selection over built or planned pieces and save it as a `[[plan]]` file in the user's plans folder, loaded like a mod's.
- Characters are assigned per (thing, material, facing), and the legend is written with them.

## Acceptance criteria

- [ ] A saved selection loads back and places the same pieces (test)
