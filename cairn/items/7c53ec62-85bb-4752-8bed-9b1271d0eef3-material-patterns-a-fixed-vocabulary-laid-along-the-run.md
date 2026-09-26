---
id: 7c53ec62-85bb-4752-8bed-9b1271d0eef3
title: 'Material patterns: a fixed vocabulary, laid along the run'
type: feature
status: backlog
milestone: houses
depends_on:
- 3fe8c3cb-dcba-4882-b623-0468ea9fe697
created: 2026-09-26
updated: 2026-09-26
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

- [ ] Every structural material in core and primitive declares a pattern, and `rim check` rejects an unknown one
- [ ] A pattern crosses a cell boundary in a run without a break (screenshot)
- [ ] Render bench: the whole-map-zoomed-out case is unchanged; the close-zoom case is within budget
