---
id: 99bbff49-0bc5-401a-8d25-1f2773f30fa5
title: CI in half the time
type: chore
status: doing
milestone: colony
assignee: Oddur Sigurdsson
claimed: 2026-09-25
created: 2026-09-24
updated: 2026-09-25
priority: p1
api: none
effort: s
layer: tooling
area: tests
---

## Why

A PR waited about ten minutes for CI, and every push ran everything twice:
`push` and `pull_request` both triggered, in different concurrency groups.
With several agents pushing, runners queue.

## What

- `push` only on main; PRs run once.
- Ubuntu's serial steps split into parallel jobs: tests, client
  (autotest, render bench), sim runs (soak, climate, bench).
- The render bench runs 100 frames a view in CI, not 300.
- The soak and climate runs go to Windows and macOS only on main; PRs run
  them on Ubuntu. The crosscheck still proves every platform agrees.
- Tests run under cargo-nextest.

## Acceptance criteria

- [ ] A PR push runs CI once
- [ ] A PR's CI wall clock before and after recorded here
