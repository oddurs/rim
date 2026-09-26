---
id: e58c8ff7-d3b8-4baa-b655-a8fcd1436af6
key: depth
title: Depth
type: milestone
status: planned
depends_on:
- dba94ebe-b56e-4605-8a77-206f66afe15d
created: 2026-09-26
updated: 2026-09-26
priority: p2
api: breaking
---

Dig down. The map becomes a stack of 2D levels joined only at stairs, ladders and holes, rock becomes terrain, and water fills what you dig into. Design: DESIGN.md §6c.

## Goal

On the default install, a colony digs a cellar at −1 with a digging stick in its first week, trenches its camp, floods the trench from the river, and watches a raid bridge it. A shaft reaches −3 by the Village era. Water from a breached aquifer fills the levels below, and colonists get out of its way. All of it inside the §8 budget, with the cost of each new system recorded in §6c.

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
