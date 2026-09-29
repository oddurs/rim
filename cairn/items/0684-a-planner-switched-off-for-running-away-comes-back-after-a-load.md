---
id: 684
uid: de02d451-3320-4d45-b9e4-de0997fe179a
title: A planner switched off for running away comes back after a load
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
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

- [ ] A switched-off planner stays off after a load
- [ ] A test that fails before the fix and passes after
