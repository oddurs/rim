---
id: 559
uid: 0777872e-fd05-4e76-8c0c-8c9aecce29f9
title: Every handler of an event gets the same Lua table
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

`dispatch_events` (script.rs ~2416-2418) calls each handler with `t.clone()`, and cloning a Luau table handle shares the table. A handler that does `e.owner = nil` changes what the next mod's handler sees.

## How it fails

Deterministic, but it breaks the isolation between mods that the module header promises, and handler order starts to matter.

## Reproduce

Not yet run; verified by reading. Two mods handle `order_done`; the first clears a field, and the second sees it cleared.

## Fix

Build the event table per handler, or freeze it (`table.freeze`) before dispatch.

## Acceptance

- [x] One handler can't change what another sees
- [x] A test that fails before the fix and passes after

## 2026-09-29

PAUSED at the merge freeze (main d633f7b5): fix and test done (test fails before, passes after), committed on fix/0777872e-event-copies and rebased on origin/main before the freeze; the full gate has not run on this commit. Next: rebase onto main, run scripts/task check, mark the PR ready.
