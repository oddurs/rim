---
id: 192
title: Snow and mud slow movement
type: feature
status: backlog
milestone: crafting
depends_on:
- 187
created: 2026-09-23
updated: 2026-09-23
priority: p3
api: additive
effort: s
layer: engine
area: pathing
pillar:
- survival
- performance
---

## Why

Deep snow should change how a winter plays: short trips, woodpiles near the door. Movement cost from fields makes that data.

## What

- `[[field]]` gains `move_cost = curve` (value to extra percent cost). Core: snow 20 cm = +50%, 50 cm = +150%; wetness above 90% on soil = +30% (mud).
- Costs are bucketed and kept in a per-cell extra-cost array, updated when a cell changes bucket, so A* reads one array. Regions are unaffected (cost never makes a cell impassable).
- Pawns walk at the cell's speed.

## Acceptance criteria

- [ ] Pawns cross deep snow measurably slower (test)
- [ ] Paths prefer a cleared route when one is close (test)
- [ ] No region rebuilds from snow (test); pathing cost measured before and after

## 2026-09-23

Moved to Crafting with snow cover (0187).
