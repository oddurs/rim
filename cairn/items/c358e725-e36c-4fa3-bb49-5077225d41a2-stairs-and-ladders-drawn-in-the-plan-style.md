---
id: c358e725-e36c-4fa3-bb49-5077225d41a2
title: Stairs and ladders drawn in the plan style
type: feature
status: backlog
milestone: houses
depends_on:
- 5689930d-2bd1-4838-b403-a72bc61c31e9
- acd85584-7f3d-4348-8d56-5242a0bdb620
created: 2026-09-27
updated: 2026-09-27
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

- [ ] The autotest gallery shows a stair and a ladder in the plan style on both levels

## 2026-09-27

Waits on the depth view (5689930d) as well as portals (merged in #225): a stair is one thing across two levels, and UP on the lower end and DN on the upper needs the renderer to know which end of the portal a cell is. That's a look mechanism (a layer shown on one end only), best added once the view draws each level.
