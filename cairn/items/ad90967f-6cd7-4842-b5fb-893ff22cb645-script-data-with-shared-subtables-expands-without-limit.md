---
id: ad90967f-6cd7-4842-b5fb-893ff22cb645
title: Script data with shared subtables expands without limit
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

`data::from_lua` (data.rs ~187-227) caps only depth (32). `local t = {} for i = 1, 30 do t = {t, t} end rim.set_data("x", t)` is a small Lua value that expands to 2^30 nodes in Rust, outside both the step budget and `MEMORY_LIMIT`. `rim.emit` goes through the same path.

## How it fails

One script call hangs the game or runs it out of memory.

## Reproduce

Not yet run; verified by reading. That snippet in a tests/scripting.rs hook.

## Fix

Count nodes against a limit, or refuse a table seen twice (by pointer) in one conversion, with a script error.

## Acceptance

- [ ] The snippet is a script error, promptly
- [ ] A test that fails before the fix and passes after
