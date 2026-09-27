---
id: d5ba1331-5fb4-4143-8219-37378748bf88
title: Delete a work role the player made
type: feature
status: backlog
milestone: mood
created: 2026-09-27
updated: 2026-09-27
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

- [ ] Deleting a role moves its members to the default role with their pins, and saves and loads (test)
- [ ] A def's role can't be deleted (test)
- [ ] The determinism test passes
