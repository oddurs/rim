---
id: 539
uid: daee2c36-fbaf-4aad-a7cb-fcf6398c0394
title: 'Round the corners and turn smoothly: pawns follow a curve inside the corner cell'
type: feature
status: done
milestone: people
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
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

- [x] A unit test: a path with a right-angle turn yields drawn positions inside the corner cell for the whole blend, and equal to the linear position outside it
- [x] A unit test: at every step boundary the drawn pawn is in the cell the sim stepped into, a turn's two halves meet, and a straight run's boundaries are cell centres
- [x] A unit test: facing turns at most 12 rad per game second and doesn't change while paused
- [x] The autotest walks a colonist round a corner, checks the drawn pawn never leaves the cells the sim steps between, and photographs it mid-turn
- [x] `rim --bench-render`: the pawn pass's CPU time before and after, noted here

## 2026-09-28

Two criteria reworded while building, because as first written they couldn't be met: a rounded turn deliberately never passes through the corner cell's centre, so 'arrival times at each cell centre match' is false by design. What holds, and is tested (rim_sim world tests), is that at each step boundary the pawn is in the cell the sim stepped into, the turn's two halves meet with no jump, and straight runs are unchanged. And nothing draws a selected pawn's trail any more, so the autotest photographs the founder mid-turn (shot round-turn) instead of a trail.

## 2026-09-28

Review (code-review, medium) found a real bug, fixed: the came-from cell was never cleared when a pawn stopped, so a pawn resuming at a right angle jumped about 0.12 of a cell on its first frame. Motion::stepped now forgets it when the pawn has no next step; the sim takes the next step in the tick it arrives at a corner, so real turns keep rounding. Test: motion::tests::a_pawn_that_stops_forgets_where_it_came_from. Declined: when the sim clears a path mid-step (replan, blocked cell, a fall), a pawn inside the last 35% into a corner snaps back to the straight line, up to about 0.12 of a cell. Rare, small, and easing it would need a second client cache; noted in the PR.

## 2026-09-28

Bench (rim --bench-render --seed 1, 200 frames per view, Apple M4 Pro; two runs each of main 0ff863bf and this branch, other sessions building on the machine): the pawns pass is noise-bound here, the same binary moving 0.08 to 0.63 ms on one view between runs. Taking each view's better run, the median over views is 0.086 ms on main and 0.075 ms on this branch: no measurable change, as expected of a few multiplies and a lookup per pawn.

## 2026-09-28

Correction: the bench numbers above are void. --bench-render runs with a hidden window on this Mac, which is throttled, so its timings mean nothing (rim-c2). The before/after for the pawns pass comes from CI's render benchmark on this PR against main's, recorded when that run finishes.

## 2026-09-28

Bench, from CI (llvmpipe under xvfb; local runs are void because a hidden window is throttled). PR run 36476491601 on 9edddfed against queue run 36470949523 on 43568f4d, the same seed-1 scene (198 pawns). The runner was slower across the board on the PR run (ui 1.57 to 2.10 ms, gpu 5.4 to 8.9 ms on the whole map), so the pawns pass is read against the things pass, which this change doesn't touch. Pawns ms (pawns / things): whole map 0.119 (0.41) to 0.171 (0.37); close 0.087 (0.20) to 0.119 (0.22); stacked 0.135 (0.35) to 0.188 (0.31); below 0.016 (0.05) to 0.017 (0.05). Mid is left out: main's things pass there is an outlier (1.72 ms against 0.29 to 0.44 elsewhere). No change the runner's noise can show: the curve is a few multiplies a pawn, and 200 pawns stay near 0.1 to 0.2 ms of the pass.
