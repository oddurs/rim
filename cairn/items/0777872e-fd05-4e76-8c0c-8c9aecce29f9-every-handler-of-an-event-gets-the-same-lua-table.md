---
id: 0777872e-fd05-4e76-8c0c-8c9aecce29f9
title: Every handler of an event gets the same Lua table
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

`dispatch_events` (script.rs ~2416-2418) calls each handler with `t.clone()`, and cloning a Luau table handle shares the table. A handler that does `e.owner = nil` changes what the next mod's handler sees.

## How it fails

Deterministic, but it breaks the isolation between mods that the module header promises, and handler order starts to matter.

## Reproduce

Not yet run; verified by reading. Two mods handle `order_done`; the first clears a field, and the second sees it cleared.

## Fix

Build the event table per handler, or freeze it (`table.freeze`) before dispatch.

## Acceptance

- [ ] One handler can't change what another sees
- [ ] A test that fails before the fix and passes after
