---
id: b3ffbae1-f129-4c30-b1f9-adb420f3cf82
title: 'Replace in place: upgrade a wall without opening the room'
type: feature
status: backlog
milestone: houses
depends_on:
- e7c4a3f6-99b4-4587-b53e-b40cb0ea96a0
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: none
effort: m
layer: engine
area: building
---

## Why

Upgrading wattle to cob for winter means taking the wall down first, and the room is open to the cold while it's rebuilt. Planning a door into a wall has the same problem. DESIGN.md §6c.

## What

- Planning a wall-group piece over another is a replacement. The old one stands while the new material is delivered, then one work session swaps them.
- A replacement's refund of the old material lands on the cell.
- It's drawn as the old piece with the new plan's hatch over it.

## Acceptance criteria

- [ ] A hut whose walls are all being replaced stays indoors until each swap (test)
- [ ] Cancelling a replacement leaves the old wall untouched
