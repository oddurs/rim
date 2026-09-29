---
id: 04bc1047-cf78-4ca2-846d-a54769c4b438
title: A planner's answer can name one work type twice
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

`read_plan` (script.rs ~108-122) resolves keys to work types. `{haul = 2, ["core:haul"] = 3}` name the same type. After `sort_unstable_by_key`, which one wins depends on Luau's `pairs` order: deterministic, but arbitrary and fragile.

## Reproduce

Not yet run; verified by reading. A planner returning both keys.

## Fix

Reject duplicates with an error the planner's mod sees.

## Acceptance

- [x] A duplicate work type in a plan is an error
- [x] A test that fails before the fix and passes after

## 2026-09-28

The item's example, haul and core:haul, can't happen: a bare id from another mod is refused ('another mod's def needs its prefix'). The duplicate is real for a mod's own work types, which have a bare and a qualified name: the test uses a planner mod with its own work type 'dig'.

## 2026-09-29

PAUSED at the merge freeze (main d633f7b5): fix and test done (test fails before, passes after), committed on fix/04bc1047-plan-names-twice and rebased on origin/main before the freeze; the full gate has not run on this commit. Next: rebase onto main, run scripts/task check, mark the PR ready.
