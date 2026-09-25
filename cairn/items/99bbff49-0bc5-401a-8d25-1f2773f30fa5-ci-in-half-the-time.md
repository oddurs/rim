---
id: 99bbff49-0bc5-401a-8d25-1f2773f30fa5
title: CI in half the time
type: chore
status: done
milestone: colony
assignee: Oddur Sigurdsson
created: 2026-09-24
updated: 2026-09-25
closed_at: 2026-09-25
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

- [x] A PR push runs CI once
- [x] A PR's CI wall clock before and after recorded here

## 2026-09-25

Done in #80, #83 and #85. PR wall clock 10-17 min (median 11.5) with up to 8 min queued, before; 8.8-11.6 min warm after #80, 9.3 after #83, queueing 2-5 s. Jobs per PR push 22 to 9. The long pole is still Windows: about 4 minutes of compiling the workspace in release on its 4-core runner.
