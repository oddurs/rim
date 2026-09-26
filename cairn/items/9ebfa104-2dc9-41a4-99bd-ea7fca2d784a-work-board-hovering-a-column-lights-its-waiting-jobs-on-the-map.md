---
id: 9ebfa104-2dc9-41a4-99bd-ea7fca2d784a
title: 'Work Board: hovering a column lights its waiting jobs on the map'
type: feature
status: backlog
milestone: building
depends_on:
- f1924f03-4122-4997-b1bd-f826e9cd3ac2
created: 2026-09-25
updated: 2026-09-25
priority: p3
api: additive
effort: s
layer: client
area: render
---

## Why

DESIGN.md §4d: hovering a Work Board column lights its waiting jobs on the map, so the player sees where the backlog is. The why panel (f1924f03) and the board's demand counts (work_waiting) know the jobs, but highlighting cells on the map is renderer work.

## What

- A UI view of the jobs waiting for a work type (their cells), from the same sources as work_waiting.
- The board reports the hovered column; the renderer outlines those cells while it's hovered.

## Acceptance criteria

- [ ] Hovering the Chop column outlines every tree marked to chop, and only those (autotest screenshot)
- [ ] Nothing is drawn when no column is hovered, and the render benchmark is unchanged
