---
id: d994346a-127f-4314-9d96-74340d3c2d76
title: 'Relief: depth into rock as a smooth fill and contours'
type: feature
status: backlog
milestone: rock-face
depends_on:
- 8fea2eef-1909-4e97-8960-4888305d0003
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: none
effort: m
layer: client
area: render
---

## Problem

A hill has no height. The plan's one light (§6c) gives walls a lit edge, and rock is as flat as the floor beside it. DESIGN.md §6g, "Height".

## Proposal

- A per-level `depth` byte per cell: a solid cell's 4-connected distance to a non-solid cell, capped at 4. Flood-filled for one chunk plus a border of 4 whenever `terrain_rev` changes for that chunk, and cached with the chunk mesh. Client-side; the sim doesn't store it.
- The chunk mesh multiplies rock fills by a smooth relief from depth: 1.0, 1.0, 0.84, 0.73, 0.65 for depth 0 to 4, interpolated between cell centres (vertex colours at cell corners, averaged from the four cells) so no cell edge shows.
- A hairline contour where depth steps up (2, 3 and 4), in the outline's broken style, at 22% ink. It fades below 6 points a cell.
- Expose `depth(p)` to the renderer for rock height (next item).

## Acceptance criteria

- [ ] A hill in the autotest's `rock_close` shot darkens toward its middle with no visible cell edges in the fill (looked at, and a pixel check that adjacent interior cells differ by less than 4%)
- [ ] Contours appear at depth 2, 3 and 4 and are gone below 6 points a cell
- [ ] Mining one cell rebuilds depth only for chunks within 4 cells of it (unit test on the rebuild set)
- [ ] Render bench: whole map unchanged within noise
