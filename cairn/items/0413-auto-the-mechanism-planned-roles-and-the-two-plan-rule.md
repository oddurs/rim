---
id: 413
uid: d41e504f-4e16-493c-bb47-28c5d25a6f05
title: 'Auto, the mechanism: planned roles and the two-plan rule'
type: feature
status: done
milestone: work
assignee: Oddur Sigurdsson
depends_on:
- 387
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p0
api: additive
effort: l
layer: engine
area: ai
---

## Why

DESIGN.md §4d: colonists start on Auto, a work role whose levels a script plans. The engine owns the mechanism (storing plans, never letting them churn, explaining them); the policy lives in a mod.

## What

- `planned = true` on a `[[work_role]]`: its members' start levels come from a plan instead of `priorities`. Core's `auto` role comes first by `order`, so new colonists join it.
- Per pawn, planned levels with a reason each: `(work, level, reason)`, plus the last proposal. A planned level changes only when two proposals in a row agree. Saved and hashed.
- `planner = "core:auto"` on a planned role names the script function that plans it: `rim.planner("core:auto", fn(board) -> plans)`. The engine calls it once an in-game hour, staggered, so the policy never needs its own clock.
- `board` is what a planner needs in one read, also available as `rim.work_board()`: work types in order with `waiting` (`ai::work_waiting`), `skill` and their `auto` table; colonists with id, name, role, skills, pins, and each work type's fixed level (role or pin) where the planner mustn't move it.
- `plans` is `{ [pawn] = { [work] = { level, reason } } }`. A 0, or a level for a pinned cell, is an error naming the planner and the work type, and that cell keeps its plan. "Auto never says never" is enforced, not hoped for.
- A mod replaces the policy by patching `work_role/core:auto` with its own `planner`, so two mods doing it is a loader conflict, not a silent last-wins.
- `auto = { per_person = n, weight = n }` on `[[work_type]]`, defaults 4 and 1, readable through `work_board`.
- `explain` gains an Auto part with its reason.

## Acceptance criteria

- [x] A planned level changes on the second agreeing proposal, not the first (test)
- [x] A planner returning 0 or a pinned cell gets an error and that cell keeps its plan (test)
- [x] A mod patching the Auto role's `planner` replaces core's in a fixture test
- [x] A colonist who leaves Auto keeps their pins and drops their plan (test)
- [x] Plans survive save and load, and the determinism test passes with a scripted planner

## 2026-09-26

A planned role is one with planner = "mod:name"; no separate planned = true flag, since a planner is what makes it planned. A work type with no plan yet takes the first proposal at once, so a new colonist isn't stuck at defaults for two hours. Reasons update whenever the proposed level matches the current one. Planner errors are posted as one message per run. Core's own auto role and planner come in f5295002, so Hand stays the default until then.
