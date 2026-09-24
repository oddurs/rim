---
id: 170
title: 'Docking shell: top, bottom, left and right regions'
type: feature
status: planned
milestone: interface
depends_on:
- 168
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: additive
effort: m
layer: client
area: ui
pillar:
- plugin-first
---

## Why

Panels need a place to live that adapts to window size and to whatever mods add, without anyone hard-coding positions.

## What

Regions at the screen edges stack the panels docked into them by order. Panels declare region, order and min/max size; regions adapt to the window size.

## Acceptance criteria

- [ ] Panels dock by declaration; order decides stacking
- [ ] Resizing the window reflows every region without overlap
- [ ] A mod adding a panel to a region pushes others aside rather than covering them
- [ ] Regions and panels have namespaced ids mods can address
