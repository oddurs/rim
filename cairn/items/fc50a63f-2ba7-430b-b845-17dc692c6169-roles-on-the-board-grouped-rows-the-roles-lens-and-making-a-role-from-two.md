---
id: fc50a63f-2ba7-430b-b845-17dc692c6169
title: 'Roles on the board: grouped rows, the Roles lens, and making a role from two'
type: feature
status: backlog
milestone: work
depends_on:
- 4c9fc19b-3b7b-4bc3-93b4-7b5d600cbc8d
- 992ecb92-f676-4a7b-af03-c6829269c6c2
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: none
effort: m
layer: core
area: ui
---

## Why

Roles only save effort if the board shows them and editing one is direct. DESIGN.md §4d: rows grouped by role; roles offered when two colonists have been pinned into the same shape.

## What

- The board groups rows by role in `order` with a header per role, and each row carries a role chip.
- A Roles lens: a card per role with its levels as shelves (tokens cycle a level; italic tokens are the work type's default), members as chips that drag between cards, and a count of members' pins.
- Right-click a colonist's name: Role ▸, "Hand everything back", "Make a role from this colonist". Right-click a cell: "Same for everyone in <role>".
- The suggestion: when two colonists in the same role share at least all but one of their pins, the board offers "Bo and Cyd work alike. Make a role from them?" once, and it sends `CreateWorkRole` and `AssignWorkRole`.
- The why panel's role select.

## Acceptance criteria

- [ ] Dragging a colonist onto a role card sends `AssignWorkRole` and keeps their pins (UI test)
- [ ] Clicking a role token changes every member without a pin (UI test)
- [ ] The suggestion appears for two matching colonists and not again after "Not now"
- [ ] Autotest screenshots of the board and the Roles lens
