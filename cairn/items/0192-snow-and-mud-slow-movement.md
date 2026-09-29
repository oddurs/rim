---
id: 192
uid: 6f1e7410-8075-431d-818f-21e3326a05a2
title: Snow and mud slow movement
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- 187
created: 2026-09-23
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] Pawns cross deep snow measurably slower (test)
- [x] Paths prefer a cleared route when one is close (test)
- [x] No region rebuilds from snow (test); pathing cost measured before and after

## 2026-09-23

Moved to Crafting with snow cover (0187).

## 2026-09-27

People (5d09b04e, DESIGN.md §6h): the stride's phase follows distance, so slower steps in snow simply take longer; nothing in the figure needs to change.

move_cost is a curve on a stock field (value to extra percent), with move_cost_by naming a terrain prop that scales it per cell. Only stock fields may have it: only they know which cells changed, so the cost stays current without a pass over the map. Mud is the weather plugin's wetness past 90% (+30%), scaled by a new core terrain prop, mud: 1 on dirt, grass, rich soil and marsh; absent (0) on sand, rock and water. No engine code names a terrain. Snow is +50% at 20 cm and +150% at 50. Map keeps extra_cost (u16 percent, every level) and Map::cost adds it, so A* and pawn steps read one array; path.rs is unchanged. Each move field keeps its cells' 10% step, and the map moves only when a step changes: this tick's stock slice, cells a script set, and cells whose terrain changed (mud follows the ground). A field with no steps yet (new map, load) is costed whole, so a loaded game walks as the live one. No revision bump and no region rebuild. Measured:
- 14 cells: 183 ticks bare, 449 in 50 cm (2.45×).
- Across a 40 cm snowfield, the path takes a shovelled lane two cells off the line.
- 3600 snowed cells over 100 ticks: 0 region rebuilds, revision unchanged.
- bench --days 2 on 250×250 with a pinned frost: 15 nodes and 0.027 ms of paths a tick at 0 cm; 39 nodes and 0.042 ms at 40 cm. The octile heuristic assumes 100%.
I didn't add a heuristic floor from the map's minimum cost: any unsnowed cell (an enclosed room, a floor) sets it back to 100, so it only helps a map snowed wall to wall. Tick totals were too noisy (other builds running) to compare.
