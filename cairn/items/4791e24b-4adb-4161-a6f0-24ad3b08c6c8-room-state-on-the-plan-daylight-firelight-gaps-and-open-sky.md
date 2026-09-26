---
id: 4791e24b-4adb-4161-a6f0-24ad3b08c6c8
title: 'Room state on the plan: daylight, firelight, gaps and open sky'
type: feature
status: backlog
milestone: houses
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

- [ ] Knocking one wall out of a hut shows the gap mark at that cell (autotest)
- [ ] Gap search stays under 1 ms on the bench map, and runs only when rooms rebuild
