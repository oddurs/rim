---
id: 334
uid: 3fe8c3cb-dcba-4882-b623-0468ea9fe697
title: 'Joins in quarters: a wall run draws as one mass'
type: feature
status: done
milestone: houses
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p0
api: additive
effort: m
layer: client
area: render
---

## Why

Walls join today by leaving out the edges that face a joined neighbour (0220). A run reads as one wall, but corners are square notches, every cell is a flat box, and the look can't tell an end from a junction. DESIGN.md §6c rules that joins are a topology, drawn in quarters.

## What

- `look.join` takes a table: `{ group, style = "mass" | "pipe" | "edges", round, connects }`. A bare string still means `{ group = s, style = "edges" }`.
- A `mass` layer reads the 8-neighbour mask. Each quarter picks outer corner, either edge, inner corner or solid, and outer corners are rounded by `round`.
- One fill per material per chunk, so joined cells never show a seam. A material change along a run draws a hairline, never an outline.
- The contour is traced once around the group, in the heaviest line weight.
- Rock joins as `rock` the same way.
- The reference is `massPath` and `massContour` in docs/engineering/houses-prototype.html.

## Acceptance criteria

- [x] Post, end, run, corner, tee, cross and a 2×2 block each draw as in the prototype (autotest screenshots)
- [x] No seam between two joined walls of one material; a hairline between two materials (pixel check, like 0220's)
- [x] A string `look.join` still loads and draws as today
- [x] Render bench within budget with every wall on the bench map as a mass

## 2026-09-26

Render bench, whole map at zoom 4: world 1.50 ms against main's 2.03 ms on the same machine (noise dominates; budget 4 ms), indices 1.50M vs 1.42M (+6%).
