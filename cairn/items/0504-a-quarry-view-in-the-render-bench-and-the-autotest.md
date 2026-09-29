---
id: 504
uid: 8fea2eef-1909-4e97-8960-4888305d0003
title: A quarry view in the render bench and the autotest
type: chore
status: backlog
milestone: rock-face
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: none
effort: s
layer: tooling
area: perf
---

## Problem

Every Rock face item changes how rock draws, and the render bench has no view where rock fills the screen at close zoom. The autotest's only rock shot is one cell in a row of worksites. Without a fixed scene there is no budget to hold the outline, patterns and ore to, and no screenshot to compare.

## Proposal

- `crates/rim_client/src/bench.rs`: a `View { name: "quarry", zoom: Some(28.0), .. }` centred on the largest rock mass on the bench map, with a 4 × 6 block mined out of its face and a mine order on the cells behind it, so the face, the order and a worksite are all in view.
- `crates/rim_client/src/autotest.rs`: a `# rock face` section with the same scene at zoom 48 and 12, one shot each (`rock_close`, `rock_mid`), and a check that the scene has at least 20 solid cells and one designated one in view.
- Record today's numbers for the quarry view in DESIGN.md §6g, "Cost".

## Acceptance criteria

- [ ] `rim --bench-render` prints a `quarry` row with CPU and GPU time
- [ ] The autotest writes `rock_close` and `rock_mid` shots and its checks pass
- [ ] Baseline numbers for the quarry view are recorded in DESIGN.md §6g
