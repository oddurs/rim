---
id: 558
uid: 04bc1047-cf78-4ca2-846d-a54769c4b438
title: A planner's answer can name one work type twice
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

`read_plan` (script.rs ~108-122) resolves keys to work types. `{haul = 2, ["core:haul"] = 3}` name the same type. After `sort_unstable_by_key`, which one wins depends on Luau's `pairs` order: deterministic, but arbitrary and fragile.

## Reproduce

Not yet run; verified by reading. A planner returning both keys.

## Fix

Reject duplicates with an error the planner's mod sees.

## Acceptance

- [ ] A duplicate work type in a plan is an error
- [ ] A test that fails before the fix and passes after
