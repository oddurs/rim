---
id: de02d451-3320-4d45-b9e4-de0997fe179a
title: A planner switched off for running away comes back after a load
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-29
closed_at: 2026-09-28
priority: p3
api: none
layer: engine
area: scripting
---

## What

`call_value` adds a runaway function's pointer to `disabled`, and `run_planners` checks it (script.rs ~2358). But `disabled()` (~2309) reports only hooks and handlers, so the snapshot loses a switched-off planner.

## How it fails

After a load, the planner runs again: a hitch of up to the whole step budget, and the message again. If it finishes this time, its plans apply where the live game's didn't: a divergence.

## Reproduce

Not yet run; verified by reading. tests/auto.rs's `rim.planner("test", ...)` with an endless loop (the tests/scripting.rs pattern). Save, load, and check a call counter kept in data.

## Acceptance

- [x] A switched-off planner stays off after a load
- [x] A test that fails before the fix and passes after

## 2026-09-29

PAUSED at the merge freeze (main d633f7b5): fix and test done (test fails before, passes after), committed on fix/de02d451-planner-off-on-load and rebased on origin/main before the freeze; the full gate has not run on this commit. Next: rebase onto main, run scripts/task check, mark the PR ready.
