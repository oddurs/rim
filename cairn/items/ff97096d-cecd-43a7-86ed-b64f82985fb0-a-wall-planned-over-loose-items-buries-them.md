---
id: ff97096d-cecd-43a7-86ed-b64f82985fb0
title: A wall planned over loose items buries them
type: bug
status: backlog
milestone: building
created: 2026-09-25
updated: 2026-09-25
priority: p2
api: none
effort: s
layer: engine
area: building
---

## What happens

`Command::Build` places a blueprint on a cell that holds an item stack, and `complete_building` raises the wall over it. The stack is left under a blocking wall: nobody can reach it, and a blueprint waiting on those very materials never gets them. It was found while testing af17cc1d: a wall row dragged across the colonist's own cell, where its wood lay, buried the wood, and the rest of the row waited forever.

## What should happen

The stack is moved off the cell before the wall goes up. It could be a haul job the construction waits on, or a nudge to the nearest free cell when the blueprint is placed.

## Reproduction

Seed: 3
Mods: core

1. Put 40 wood on a cell.
2. Plan a wood wall row through that cell.
3. The wall on the wood's cell is built over it, and the others never get their wood.

## Acceptance criteria

- [ ] No completed building stands over an item stack
