---
id: 522
uid: c358e725-e36c-4fa3-bb49-5077225d41a2
title: Stairs and ladders drawn in the plan style
type: feature
status: done
milestone: houses
assignee: Oddur Sigurdsson
depends_on:
- 346
- 393
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p2
api: none
effort: s
layer: core
area: render
---

## Why

Portals (acd85584) give stairs and ladders a plain look. A plan draws a stair as treads with a break line and an UP or DN arrow, and a ladder as rungs (DESIGN.md §6c). The Depth session left the symbol to the houses milestone.

## What

- Core's stairs and ladder looks use the plan vocabulary (`box`, `line`, the weights): treads across the run, a break line, and an arrow saying which way the level is.
- The arrow follows the viewed level: UP on the lower level, DN on the upper.

## Acceptance criteria

- [x] The autotest gallery shows a stair and a ladder in the plan style on both levels

## 2026-09-27

Waits on the depth view (5689930d) as well as portals (merged in #225): a stair is one thing across two levels, and UP on the lower end and DN on the upper needs the renderer to know which end of the portal a cell is. That's a look mechanism (a layer shown on one end only), best added once the view draws each level.

## 2026-09-27

A layer can say on = "top" or "bottom" to show on one end of a portal only; the renderer now draws a portal's bottom end (it held the stair in the map but was never drawn, since things draw from their anchor). Core's stairs: treads, the break line on the top end, and an arrow DN at the top, UP at the bottom; the ladder: rails and rungs with the same arrows. Draw test: the bottom end draws, with its UP arrowhead on top, and it fails without the change. Autotest 303/303 with the gallery shot on both levels.
