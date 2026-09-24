---
id: 219
title: 'Furniture: table, chair, stove'
type: content
status: backlog
milestone: building
depends_on:
- 214
- 218
created: 2026-09-23
updated: 2026-09-23
priority: p2
api: none
effort: s
layer: core
area: building
---

## Why

The room needs things in it.

## What

- `table`, `chair`, `stove`, built of any material like everything else.
- Chairs take a spot at an adjacent table; pawns eat there when they can.
- The stove emits heat and light, with a cap, like the campfire.

## Out of scope, deliberately

- **Mood.** A table's real job in this genre is "ate at a table" against
  "ate on the floor", and that needs 0088. Flagged and accepted: the
  furniture ships now and gains its effect when mood lands. Raised before
  the sprint was planned.
- **Cooking.** A stove that cooks needs bills and recipes, which
  DESIGN.md 5 puts outside core. Until then it is a fancy campfire.

## Acceptance criteria

- [ ] All three buildable in every material
- [ ] Pawns choose a chair at a table to eat
- [ ] The stove heats a room to its cap and no further
