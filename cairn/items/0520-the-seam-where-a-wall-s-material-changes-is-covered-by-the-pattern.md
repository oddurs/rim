---
id: 520
uid: c2d73146-e691-4ad3-b3a4-061099b42ed1
title: The seam where a wall's material changes is covered by the pattern
type: bug
status: done
milestone: houses
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p1
api: none
effort: s
layer: client
area: render
---

## Why

Where wood meets stone in a run of wall, a thin dark seam marks the change of material (DESIGN.md §6c). The seam is drawn inside `mass`, and the `pattern` layer paints after it, so where stone's mortar joint falls on the cell's west edge, the joint covers the seam. Patterns are laid in world space, so it depends on where the wall stands: the autotest's hairline check passed or failed with the map.

## What

- Draw the seam after the pattern, so nothing in the wall covers it.

## Acceptance criteria

- [x] A test paints wood beside stone at every pattern offset and finds the seam on top at the joint

## 2026-09-27

Found through #208: adding core defs shifted maps under the old def-index spawn hashing, which moved the autotest's wood-and-stone row onto an offset where a mortar joint covered the seam. Fixed by drawing the seam once, after all of a cell's layers, in its mass colour. The draw test records what is painted in order and checks the top colour at the joint for 23 joints along a row, at 7 heights each.
