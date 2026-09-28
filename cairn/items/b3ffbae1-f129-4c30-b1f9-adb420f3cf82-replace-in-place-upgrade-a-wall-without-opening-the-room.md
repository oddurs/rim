---
id: b3ffbae1-f129-4c30-b1f9-adb420f3cf82
title: 'Replace in place: upgrade a wall without opening the room'
type: feature
status: done
milestone: houses
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] A hut whose walls are all being replaced stays indoors until each swap (test)
- [x] Cancelling a replacement leaves the old wall untouched

## 2026-09-27

Built on the existing Blueprint machinery rather than work orders (e7c4a3f6): a replacement is a blueprint off the fixture layer carrying Replaces(old). Delivery and construction find blueprints through the ECS, so both work unchanged, and complete_building swaps them in one step. The old piece's refund is placed once the new one holds the cell. The rule for what replaces what: same join group, one cell each, and either a new material (in any drag) or a different piece (only on a single click, or from a house plan), so drawing a room again upgrades its walls but keeps its door. If the old piece goes some other way first, the replacement becomes an ordinary plan.
