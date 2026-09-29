---
id: 715
uid: 7f3dd8e3-fdcf-4e49-b140-8098569041cb
title: Carrying and equipping
type: milestone
status: backlog
created: 2026-09-29
updated: 2026-09-29
key: carrying
priority: p1
api: additive
---

A colonist's hands, load and clothes become the player's to direct, and a load has weight. Right-click a hammerstone to equip it, a hide wrap to wear it, a stack to haul it, a crafting spot to start a bill. Heavy things come in small loads and slow the walk; berries come by the armful.

The design is in the Carrying and equipping doc: https://claude.ai/code/artifact/349db585-8e04-44a3-a729-f2163bd8d5f4. The user asked for it on 2026-09-29 after playing the frozen build, and settled its questions the same day:

- **A chosen tool and work that needs another:** swap and return. The colonist puts the chosen tool down, uses the other, then picks the chosen one back up.
- **Weight:** yes. Stacks are sized by mass, and a heavy load walks slower.
- **Backpacks:** later in the game, as crafted content (in crafting).
- **Drafting:** a drafted colonist drops the carried stack, as today.

## Goal

With a colonist selected, right-click offers equip, wear, haul, drop, take off and craft now. The inspector shows Hands, Carrying and Worn. A colonist hauling stone makes more trips, and walks slower, than one hauling berries. Tick time at 250² with 50 colonists is unchanged on the scaling bench.

## Order

The two bugs first (a load dropped walking off the map, worn garments unchecked on load). Then weight, which changes every haul. Then the orders with the chosen mark, then craft now and the inspector rows.
