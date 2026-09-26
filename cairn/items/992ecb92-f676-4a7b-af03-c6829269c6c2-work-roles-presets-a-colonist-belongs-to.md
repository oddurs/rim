---
id: 992ecb92-f676-4a7b-af03-c6829269c6c2
title: 'Work roles: presets a colonist belongs to'
type: feature
status: done
milestone: work
assignee: Oddur Sigurdsson
depends_on:
- 166a4cc9-fee8-4cd4-9384-4ec243f74031
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

"Builders haul sooner" is one intent and costs one cell per builder on the grid. A new settler costs a cell per work type. DESIGN.md §4d: work roles are partial sets of levels colonists belong to, one each, and a colonist's own setting (a pin) beats the role.

## What

- `[[work_role]]` defs: `id`, `label`, `order`, `priorities = { build = 1, … }`, `planned = false` (Auto sets it true later). Named work roles so they never read as room roles (f709cdd4). Patchable as `work_role/<mod>:<id>`.
- Roles are seeded from defs into the world at a new game, and on load for a role a mod added since. The colony's copy records whether the player edited it; an unedited one follows its def on load.
- `Pawn` gains `work_role`. A colonist without one (an old save) takes the first role by `order`, and so does every new colonist.
- Resolution: `start = role's level | work type default`, `base = pin | start`, then rules (`rules::eval` unchanged below the base). `explain` gains a role part, and the pin part is labelled as the colonist's.
- Commands: `AssignWorkRole { pawn, role }` (pins stay), `SetRolePriority { role, work, level | inherit }`, `CreateWorkRole { label, from: Role(id) | Pawn(e) }`. All saved, hashed, in the text save.
- Luau: `rim.work_role(pawn) -> string`, `rim.work_roles() -> {…}`; `PriorityPart` gains `kind`.
- Core content: `builder` (build 1, mine 2, haul 2, hunt 0) and `forager` (harvest 1, hunt 1, chop 2), plus a plain `hand` role that sets nothing and comes first until Auto replaces it. The crafting mod ships `crafter` and patches `builder` with craft at the last level.

## Acceptance criteria

- [x] Editing a role moves every member without a pin, and not a member with one (test)
- [x] A role a mod updates follows the update on load unless the player edited it (test)
- [x] An old save loads with everyone in the first role and the same effective levels as before (test)
- [x] `explain` lists default, role, pin and rules, and the parts sum to the level (test)
- [x] Save round trip, text save and determinism tests pass
