---
id: 486
uid: 5ea1df47-1cd8-49e4-9a3c-4629b541945d
title: A pawns pass in rim --bench-render, held to its budget in CI
type: feature
status: done
milestone: people
assignee: Oddur Sigurdsson
depends_on:
- 480
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
priority: p0
api: none
pillar:
- performance
effort: s
layer: tooling
area: perf
---

## Why

DESIGN.md §8 gives the world renderer 4 ms of CPU on the reference machine,
and §6h gives pawns 0.4 ms of it for 200 at full detail. `rim --bench-render`
measures per pass, and CI fails over budget, but pawns aren't a pass of their
own yet, so a figure that got slower would only show up in the total.

## What

- `--bench-render` reports a `pawns` pass: CPU time, instances and draw
  calls, with 200 pawns walking, carrying and working, at each level of
  detail.
- The CI budget check covers it: over 0.4 ms at full detail, or more than one
  draw call, fails.
- The render bench's scene gains the 200 pawns if it has fewer.

## Acceptance criteria

- [x] `rim --bench-render` prints the pawns pass with CPU, instances and draw calls at 7, 15 and 40 px a cell
- [x] CI fails a deliberately slowed pawn pass (shown once on a throwaway PR, linked here)
- [x] The before and after numbers for 535a1fb9 are recorded here

## 2026-09-28

The spike (cc289956) measured macOS only, 200 eight-part figures at 0.087 ms of CPU and one draw call with the distance-field batch. This pass is where Linux and GPU time get measured, in CI, on the real batch.

## 2026-09-28

CI (run 36491129977 on #352, llvmpipe): the crowd of 200 is 0.076 ms of pawns-pass CPU as dots (235 parts), 0.143 ms as silhouettes (700) and 0.109 ms in full (1,968), one draw call each, against 0.4 ms. Since #340, handing the batch to GL counts as submission, not as the pass.

## 2026-09-28

535a1fb9 before and after, from the queue's A/B (main 36480969471 against the branch 36480974094) and two no-bodies runs on the branch run's runner class (36480300758, 36480273417). The first A/B crossed runner classes; the moving light pass is about 3.8 ms on one class and 1.35 ms on the other. World + ui, same class, no bodies (two runs) against bodies: whole map 1.82 and 2.15 against 1.99; mid 2.96 and 2.96 against 2.50; close 1.42 and 1.34 against 1.61; moving 5.63 and 5.79 against 6.03; dusk 2.88 and 2.92 against 2.21. Draw calls: one more per view, the batch's own; the discs had ridden in macroquad's batches.

## 2026-09-28

CI fails a slowed pass: throwaway #353 (#352 plus a 1 ms spin at the top of draw::pawns) failed its render benchmark with 'render bench: 200 figures in full take 1.144 ms of CPU, over the budget of 0.60 ms' (run 36501546489, job https://github.com/oddurs/rim/actions/runs/36501546489/job/109193323146). #353 is closed unmerged.
