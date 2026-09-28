---
id: daee2c36-fbaf-4aad-a7cb-fcf6398c0394
title: 'Round the corners and turn smoothly: pawns follow a curve inside the corner cell'
type: feature
status: backlog
milestone: people
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: none
pillar:
- performance
effort: s
layer: client
area: render
---

## Why

A pawn moves from cell centre to cell centre in straight steps
(`Pawn::drawn_at`, `ai.rs` `advance_movement`), so it snaps 90° at every
corner and its facing, when figures have one, would jump. This is the most
visible thing about how a colonist moves. DESIGN.md §6h, "Tension: sharp
corners or round ones?"

## What

Render only; the sim is untouched and nothing feeds back (§6a).

- The client keeps, per pawn entity, the cell it came from. It's rebuilt from
  nothing on load, and a pawn with none draws as today.
- Around a corner cell, the last and first 35% of the two steps become one
  quadratic curve through the corner cell's centre. The sim's distance along
  its steps maps to distance along the curve, so the pawn arrives at each
  cell exactly when the sim says. Straight runs and diagonal runs are
  unchanged.
- The curve lies inside the triangle of the two trimmed ends and the centre,
  so it never leaves the cell the sim says the pawn is in.
- Facing: the direction of the drawn motion, turned at most 12 radians per
  game second, so a U-turn takes a visible moment. Working, a pawn faces its
  worksite (`worksite.rs` already knows `toward`); idle, it keeps its last
  facing.
- Facing and the curve use game time, so a paused game holds still and 6× is
  coherent.
- Until bodies land (535a1fb9), today's disc is drawn
  at the curved position, and facing has no visible effect.

## Acceptance criteria

- [ ] A unit test: a path with a right-angle turn yields drawn positions inside the corner cell for the whole blend, and equal to the linear position outside it
- [ ] A unit test: arrival times at each cell centre match the sim's `progress` exactly
- [ ] A unit test: facing turns at most 12 rad per game second and doesn't change while paused
- [ ] The autotest walks a colonist round a corner, with a screenshot showing the curved trail
- [ ] `rim --bench-render`: the pawn pass's CPU time before and after, noted here
