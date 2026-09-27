---
id: e77facff-4759-444d-88ef-361da5b85cc8
title: 'The mining worksite: fresh break, fracture, sparks and hewn floor'
type: feature
status: backlog
milestone: rock-face
depends_on:
- 8fea2eef-1909-4e97-8960-4888305d0003
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: additive
effort: m
layer: client
area: render
---

## Problem

Mining shows three cracks, chips and dust (§6b), and a mined cell leaves plain `rock_floor`. Stage 6 looks like stage 3, a pick sounds like a maul, and a dug gallery looks like natural ground. DESIGN.md §6g, "The work".

## Proposal

- `wear = "cracks"` gains two layers in `wear.rs`: fresh break (up to three pale flakes on the worked side, at 1.38 × the rock's colour, one more per 0.22 of progress) and, from 6/8, two fractures right across the cell. The cell never shrinks (§6b).
- A strike effect `sparks` in the `[[work_style]]` vocabulary, with `sparks = "<tool tag>"`. A strike throws five sparks when the worker's tools (`World::tool_tags`) include that tag and the thing is `hard = true`. Core: `mining.strike += "sparks"`, `sparks = "metal"`, granite and limestone `hard`.
- Chips use each yield's colour at its share, so chalk with flint throws the odd black chip.
- A floor pattern `hewn`: six short pick strokes in one of two diagonals by the cell's hash, and grit. Core: a `hewn_floor` terrain (colour of `rock_floor`), which becomes granite's, limestone's and chalk's `solid.leaves`.

## Acceptance criteria

- [ ] Stages 0 to 8 of one cell look different at each step (autotest strip of nine shots, looked at)
- [ ] With an iron pick on granite there are sparks; with a stone maul there are none (autotest particle count)
- [ ] A mined granite cell leaves `hewn_floor` with the hewn pattern (test on the terrain, shot)
- [ ] Determinism test passes (a terrain changes what mining leaves)
