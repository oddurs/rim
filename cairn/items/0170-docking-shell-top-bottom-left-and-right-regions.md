---
id: 170
title: 'Docking shell: top, bottom, left and right regions'
type: feature
status: done
milestone: interface
depends_on:
- 168
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
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

- [x] Panels dock by declaration; order decides stacking
- [x] Resizing the window reflows every region without overlap
- [x] A mod adding a panel to a region pushes others aside rather than covering them
- [x] Regions and panels have namespaced ids mods can address

## 2026-09-23

top/bottom/left/right regions; left/right stack 'start' from the top and 'end' from the bottom. Shell test checks every panel sits against its edge and reflows to a new viewport. Verified by `cargo test -p rim_ui` (15 engine tests, headless) and `rim --autotest` (92/92, screenshots reviewed).
