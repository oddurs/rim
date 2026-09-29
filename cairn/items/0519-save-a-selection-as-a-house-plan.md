---
id: 519
uid: c281689c-26c5-4352-9299-4cda3d307bd0
title: Save a selection as a house plan
type: feature
status: done
milestone: houses
assignee: Oddur Sigurdsson
depends_on:
- 373
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] A saved selection loads back and places the same pieces (test)

## 2026-09-27

rim_sim::plan::plan_text writes the fixtures in a rectangle as a [[plan]] (legend characters per thing, material and facing, the first letter of the name where free; multi-cell pieces over their whole footprint). The test round-trips the shed through text and a new mod. The client's Save as plan tool writes plans/plan_N.toml in the player's data folder and says where; it deliberately doesn't write into the mods folder, which tests load. Loading the player's own files as defs is the mod manager's job (7f26e2e3). The order toast gained 'undoable' so a save shows no Undo button.
