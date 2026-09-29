---
id: 452
uid: 202b16c4-9176-4b6f-a85a-7c45b13427e1
title: Draw the water in basins, by depth
type: feature
status: done
milestone: depth
assignee: Oddur Sigurdsson
depends_on:
- 330
- 346
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
priority: p2
api: none
effort: s
layer: client
area: render
---

## Why

Split from 3979868c (basins): a flood nobody can see is a trap, not a hazard (DESIGN.md §6d).

## What

- Wet cells of the viewed level drawn over the ground, darker with depth, from one texture per level that updates when a basin's volume or front moves.
- The level below's water shows through air, dimmed, like the rest of it.

## Acceptance criteria

- [x] A flooding mine is drawn filling, ring by ring (autotest shot)
- [x] Render bench: a flooded level costs inside the 4 ms budget, recorded here

## 2026-09-28

Render bench (M4 Pro, --sprite-mods 20, 100 frames, load ~30): the 'below' view, now a flooded level, is 0.054 ms of world CPU a frame and 'stacked' 0.24 ms, against 4 ms; --check passes. Water is one texture a level, redone at most 10 times a second while the sim's water revision for it moves. Found while benching: on seed 1 the bench's stacked room lies under a lake, so since basins merged it floods from above the moment it is dug, and the three colonists it puts there start in deep water and drown (deep water is no footing, so they can't walk out). That's the rules as designed; the bench also breaks a river into the room's wall so it floods by intent, not by seed.
