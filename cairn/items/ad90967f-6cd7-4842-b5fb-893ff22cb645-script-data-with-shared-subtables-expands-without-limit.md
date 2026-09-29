---
id: ad90967f-6cd7-4842-b5fb-893ff22cb645
title: Script data with shared subtables expands without limit
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

`data::from_lua` (data.rs ~187-227) caps only depth (32). `local t = {} for i = 1, 30 do t = {t, t} end rim.set_data("x", t)` is a small Lua value that expands to 2^30 nodes in Rust, outside both the step budget and `MEMORY_LIMIT`. `rim.emit` goes through the same path.

## How it fails

One script call hangs the game or runs it out of memory.

## Reproduce

Not yet run; verified by reading. That snippet in a tests/scripting.rs hook.

## Fix

Count nodes against a limit, or refuse a table seen twice (by pointer) in one conversion, with a script error.

## Acceptance

- [x] The snippet is a script error, promptly
- [x] A test that fails before the fix and passes after

## 2026-09-28

A node budget (data::MAX_VALUES, 2^20 values per conversion) rather than refusing a table seen twice: a shared subtable is legitimate plain data (one empty table used for many entries), only its expansion is the problem. The test uses 20 levels (2 million values, twice the budget), which completes before the fix; the item's 30 would take a billion allocations to show failing.

## 2026-09-29

PAUSED at the merge freeze (main d633f7b5): fix and test done (test fails before, passes after), committed on fix/ad90967f-data-expansion and rebased on origin/main before the freeze; the full gate has not run on this commit. Next: rebase onto main, run scripts/task check, mark the PR ready.
