---
id: 370
uid: 7c53ec62-85bb-4752-8bed-9b1271d0eef3
title: 'Material patterns: a fixed vocabulary, laid along the run'
type: feature
status: done
milestone: houses
assignee: Oddur Sigurdsson
depends_on:
- 334
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: additive
effort: m
layer: client
area: render
---

## Why

A cob wall and a stone wall differ only in tint. A plan tells materials apart by their hatch, and Minecraft tells them apart by their surface. Either way, no sprite is needed. DESIGN.md §6c.

## What

- `stuff.look = { pattern }`, with a pattern from a fixed set: `weave`, `stipple`, `logs`, `rubble`, `courses`, `bond`, `crag`, `none`. Floors get their own: `planks`, `flags`, `earth`, `rushes`, `cobbles`.
- A `pattern` layer draws the material's pattern clipped to the mass, in world coordinates and along the run, so it flows from cell to cell.
- Hairline weight, in the material's colour. It fades out below a zoom level, and zoomed all the way out it costs nothing.
- The reference is `pattern` and `floorPattern` in the prototype.

## Acceptance criteria

- [x] Every structural material in core and primitive declares a pattern, and `rim check` rejects an unknown one
- [x] A pattern crosses a cell boundary in a run without a break (screenshot)
- [x] Render bench: the whole-map-zoomed-out case is unchanged; the close-zoom case is within budget

## 2026-09-26

Render bench: whole map at zoom 4 draws no patterns (world 0.9-1.0 ms); mid (zoom 12) is below the fade, 0.77 ms; close (zoom 28) 0.78-0.92 ms, about +30% indices. Budget 4 ms whole map. The fade starts at 14 points a cell: at 10, mid zoom paid ~35% more indices for hairlines too faint to read.
