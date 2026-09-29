---
id: 280
uid: 9ebfa104-2dc9-41a4-99bd-ea7fca2d784a
title: 'Work Board: hovering a column lights its waiting jobs on the map'
type: feature
status: backlog
milestone: mood
depends_on:
- 533
- 257
created: 2026-09-25
updated: 2026-09-27
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

## 2026-09-26

Moved from building, which had already shipped when this was filed: with the other Work Board follow-ups.

## 2026-09-27

Draw the spotlight with Chalkline (DESIGN.md §6f): a veil (theme color.veil) over the map, with a hole for each waiting job, and a 1.5 px edge in the work type's hue on a keyline. It depends on the overlay palette item for the tokens and primitives.

## 2026-09-27

This item adds the theme token veil (and its doc row) to mods/core/ui/theme.toml and docs/modding/ui.md: tokens land with the item that first reads them (d83192ed review).
