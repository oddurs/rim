---
id: e2c56c9a-f105-4081-a5d9-b11635616338
title: UI frame-budget test asserts wall-clock time on shared CI runners
type: bug
status: review
milestone: proving-ground
assignee: Oddur Sigurdsson
claimed: 2026-09-27
created: 2026-09-24
updated: 2026-09-27
priority: p0
api: none
effort: s
layer: tooling
area: tests
---

## What happens

`rim_ui/tests/engine.rs::whole_ui_fits_the_frame_budget_with_30_colonists`
asserts `median rebuild frame < 2.0 ms` in wall-clock time. On shared CI
runners it lands on both sides of that line for the same commit: on PR #20
one macOS job measured 2.440 ms and one Ubuntu job 2.058 ms while their
sibling jobs, same binary, passed. Locally the same test reads 0.87-0.96 ms
on both `main` and the branch, so the failures were the runner, not the
change.

## What should happen

A budget test should fail only when the code got slower. Options, cheapest
first: assert against a ratio to a baseline measured in the same process
(paint-only vs rebuild) rather than an absolute; or mark the absolute
assertion `#[ignore]` on CI and keep it as a local check; or move it to the
`0198`-style harness with a recorded number and a tolerance.

## Reproduction

Seed: n/a
Mods: core, wildlife_plus, weather
Tick: n/a

1. Push any commit; watch `Test (macos-latest)` and `Test (ubuntu-latest)`.
2. Roughly one job in four fails this test with a median just over 2 ms.
3. `gh run rerun --failed` passes.

## Acceptance criteria

- [x] The test cannot fail on a slow runner without the code being slower
- [ ] Ten consecutive CI runs on an unchanged commit are green

## 2026-09-24

Filed from the Building sprint (0215). Same class as 0136: CI rolls dice on something unrelated to the change under test. Two CI cycles on PR #20 were spent proving the branch was not slower (it is 5% faster locally).

## Proposed status: backlog -> dropped (Oddur Sigurdsson, 2026-09-24)

Superseded by #23 (3d06bcd): the test now multiplies its budgets by a CI slack factor, which is the first of the three fixes this item listed. A slow enough runner can still trip it, so the criteria are not strictly met -- dropping is a judgement, hence a proposal rather than a close.

## 2026-09-27

The frame-budget tests assert the rebuild count (the work) always, and their wall-clock budgets only with RIM_BUDGETS=1, which CI's Linux test job sets in its own step running just these four tests one at a time. So in the suite a slow runner can't fail them (criterion 1). Measured alone under load average ~88: median frames 0.53-0.74 ms against the 1 ms budget, machine factor 1.2-1.7. Criterion 2 needs ten CI runs on main after merge; this stays in review until they're green, then closes.
