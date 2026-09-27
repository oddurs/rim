---
id: 4791e24b-4adb-4161-a6f0-24ad3b08c6c8
title: 'Room state on the plan: daylight, firelight, gaps and open sky'
type: feature
status: doing
milestone: houses
assignee: Oddur Sigurdsson
claimed: 2026-09-26
depends_on:
- 24100bb9-a9f8-430b-9c0a-4b8b7ed4dfb9
- 7129a537-c108-4a16-8f0f-27a5cd9419d8
created: 2026-09-26
updated: 2026-09-26
priority: p2
api: none
effort: m
layer: client
area: render
---

## Why

The look never lies (§6b). What the sim knows about a room should be visible without an overlay. DESIGN.md §6c.

## What

- A window facing an enclosed room throws a fan of daylight into it, scaled by its `pass`.
- A fire's warm wash is clipped to its room.
- A gap: a missing piece in a wall run that, filled, would enclose a room. It is marked with a dashed line. The client finds it by flooding from both sides with the cell blocked, only when rooms rebuild.
- Unroofed floor inside walls is hatched as sky.

## Acceptance criteria

- [x] Knocking one wall out of a hut shows the gap mark at that cell (autotest)
- [x] Gap search stays under 1 ms on the bench map, and runs only when rooms rebuild

## 2026-09-26

Window fans and fire washes move to lighting (2f13e01d, PR #151), drawn as light. This item keeps only the gap marker and the open-sky hatch.

## 2026-09-26

Gap search: 0.32 ms over a field of 59 broken huts (unit test, loaded machine), only when rooms rebuild. A candidate is open floor between two built pieces in a room that reaches the edge; it is a gap if, blocked, a side floods to fewer than 2000 cells without reaching the edge. Map::room_cell is now public so the client floods by the rooms' own rule.
