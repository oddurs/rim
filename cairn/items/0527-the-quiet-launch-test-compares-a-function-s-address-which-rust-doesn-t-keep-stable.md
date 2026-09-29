---
id: 527
uid: ceed6be9-a863-40e2-b6de-bb5b1ebec19a
title: The quiet-launch test compares a function's address, which Rust doesn't keep stable
type: bug
status: done
milestone: interface
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p2
api: none
effort: s
layer: client
area: tests
---

## What happens

## What should happen

## Reproduction

Seed:
Mods:
Tick:

1.

`quiet::mac::tests::activating_the_app_is_declined` compared the runtime's
implementation with `decline as *const c_void`. Rust promises a function
no single address across codegen units, so on branches that grow
`rim_client` the two can differ and the test fails with nothing wrong
(reported on two sessions' branches, release builds on macOS 26).

## Acceptance criteria

- [x] The test compares against the pointer `install` registered
- [x] The test calls the installed method and sees it decline
