---
id: 532
uid: d5ba1331-5fb4-4143-8219-37378748bf88
title: Delete a work role the player made
type: feature
status: done
milestone: mood
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p3
api: additive
effort: m
layer: engine
area: ai
---

## Why

Colonists name work roles by index into the colony's list (992ecb92), so a role can't be removed without renumbering everyone. A player who makes "Crew 3" by accident keeps it forever.

## What

- `DeleteWorkRole { role }` for a player's own role (not a def's). Its members move to the default role, keeping their pins. Indices above it shift down in every colonist's `work_role`, or the slot becomes a tombstone the list skips. Pick one and say why.
- The Roles lens and the colonist menu offer it, with a confirm step inside the UI.

## Acceptance criteria

- [x] Deleting a role moves its members to the default role with their pins, and saves and loads (test)
- [x] A def's role can't be deleted (test)
- [x] The determinism test passes

## 2026-09-27

Shift, not tombstones: the roles after a deleted one move down and every pawn's work_role follows. The list stays exactly what view.board().roles and rim.work_roles() index, with no hole to skip in the UI, scripts or the save, and no new field in the save. The cost: a role index held across ticks can point at a different role, which docs/modding/work.md now warns about. DeleteWorkRole is the last Command variant, so saved command logs keep their numbers. The confirm lives in the Roles card (a question under its head, since rows don't wrap); the colonist menu opens Work on that question.
