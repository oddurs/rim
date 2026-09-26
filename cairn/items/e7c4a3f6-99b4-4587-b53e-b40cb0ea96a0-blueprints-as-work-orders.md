---
id: e7c4a3f6-99b4-4587-b53e-b40cb0ea96a0
title: Blueprints as work orders
type: feature
status: backlog
milestone: crafting
created: 2026-09-25
updated: 2026-09-25
priority: p3
api: none
effort: m
layer: engine
area: building
---

## Why

Work orders (74b6fa7e) and blueprints are the same job: bring these things to a place, then work there. Construction still has its own Blueprint, Deliver and Construct. One mechanism would give building what orders have (inputs by tag, a tool requirement) and orders what building has (the deliver-first scoring, staged looks).

## What

- A blueprint becomes an `Order` whose completion is the engine's own: it becomes the building.
- `Deliver` and `Construct` fold into `Supply` and `Craft`, with no change in behaviour for building.
- Saves carry forward: `engine:blueprint` is read into orders on load.

## Acceptance criteria

- [ ] Every construction and deconstruction test passes unchanged
- [ ] A blueprint can need a tool, through the same `requires`
- [ ] Old saves with blueprints load
