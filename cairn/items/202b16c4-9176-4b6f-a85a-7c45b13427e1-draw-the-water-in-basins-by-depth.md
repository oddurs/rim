---
id: 202b16c4-9176-4b6f-a85a-7c45b13427e1
title: Draw the water in basins, by depth
type: feature
status: backlog
milestone: depth
depends_on:
- 3979868c-6de5-4926-8277-b4402adab473
- 5689930d-2bd1-4838-b403-a72bc61c31e9
created: 2026-09-27
updated: 2026-09-27
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

- [ ] A flooding mine is drawn filling, ring by ring (autotest shot)
- [ ] Render bench: a flooded level costs inside the 4 ms budget, recorded here
