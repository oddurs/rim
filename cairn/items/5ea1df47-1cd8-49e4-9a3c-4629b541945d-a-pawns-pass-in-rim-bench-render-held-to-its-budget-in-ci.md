---
id: 5ea1df47-1cd8-49e4-9a3c-4629b541945d
title: A pawns pass in rim --bench-render, held to its budget in CI
type: feature
status: backlog
milestone: people
depends_on:
- 535a1fb9-2cd8-4798-b31c-04f89fb1aee0
created: 2026-09-27
updated: 2026-09-27
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

- [ ] `rim --bench-render` prints the pawns pass with CPU, instances and draw calls at 7, 15 and 40 px a cell
- [ ] CI fails a deliberately slowed pawn pass (shown once on a throwaway PR, linked here)
- [ ] The before and after numbers for 535a1fb9 are recorded here
