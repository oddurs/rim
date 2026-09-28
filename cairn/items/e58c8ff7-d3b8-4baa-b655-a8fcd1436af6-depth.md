---
id: e58c8ff7-d3b8-4baa-b655-a8fcd1436af6
key: depth
title: Depth
type: milestone
status: done
depends_on:
- dba94ebe-b56e-4605-8a77-206f66afe15d
created: 2026-09-26
updated: 2026-09-28
closed_at: 2026-09-28
priority: p2
api: breaking
---

Dig down. The map becomes a stack of 2D levels joined only at stairs, ladders and holes, rock becomes terrain, and water fills what you dig into. Design: DESIGN.md §6d.

## Goal

On the default install, a colony digs a cellar at −1 with a digging stick in its first week, trenches its camp, floods the trench from the river, and watches a raid bridge it. A shaft reaches −3 by the Village era. Water from a breached aquifer fills the levels below, and colonists get out of its way. All of it inside the §8 budget, with the cost of each new system recorded in §6d.

## Order

1. **Rock is terrain.** Worth shipping alone: it takes every rock cell out of the ECS on today's maps. The crafting milestone's mining ticket builds on it.
2. **The level stack.** Positions gain `z`; untouched levels cost nothing.
3. **Strata.** Generate a level from the seed and `z` the tick something digs in.
4. **Portals, reach and pathing across levels.** Stairs, ladders, dig down.
5. **The view.** `[` and `]`, the depth ruler, the level below through air.
6. **Air, pits and bridges.** Trenches as defence, raiders who bridge them.
7. **Basins.** The water table, aquifers, flooding and drowning.
8. **Underground warmth and dark.**

## Not in this milestone

- Building up (floors on walls, roofs, wall walks): Defense.
- The caverns at −4 and what comes up from them: a plugin, in World.
- Ore veins, rock kinds and prospecting: Crafting (`db7f1e06`), on top of 1.
- Rain runoff and puddles: Scale (`1415721a`), a stock-field plugin.

## Due

Not set. Where it sits relative to Defense and Crafting is a person's call.

## 2026-09-28

Shipped, 13 items: rock is terrain (#157), the level stack (#177), strata (#212), portals (#225), the view (#245), pits and bridges (#255), basins (#268), water depth (#283), underground warmth and dark (#294), drawing water (#296), the scramble out of deep water (#301), movement classes (#304), and the ruler's alerts (#272, #280). Against the goal: a colony can dig a cellar at -1 with a digging tool in its first days (the balance bot does by day 2), trench its camp, flood it from the river, and watch a raid bridge it; a breached aquifer fills the levels below first; colonists leave rising water, and a pawn caught deep scrambles out. Measured inside the budget and recorded in DESIGN.md §6d: the flooded level draws at 0.05 ms a frame; water at rest ~0.1 µs a tick. Not met: the cellar alone doesn't carry a colony through winter (it holds ~9°C under the warmth need's 10°C comfort; the balance numbers are on f2a8ffc7), and the basin rebuild timings were taken at load 100+ and want re-measuring on a quiet machine. Left open elsewhere: defences in the raid budget (5f022ef4), incremental regions and rooms (66906291). Depth closed before Scale, which it was ordered after.
