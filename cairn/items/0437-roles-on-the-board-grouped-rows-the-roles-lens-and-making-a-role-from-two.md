---
id: 437
uid: fc50a63f-2ba7-430b-b845-17dc692c6169
title: 'Roles on the board: grouped rows, the Roles lens, and making a role from two'
type: feature
status: done
milestone: work
assignee: Oddur Sigurdsson
depends_on:
- 340
- 387
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
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

- [x] Dragging a colonist onto a role card sends `AssignWorkRole` and keeps their pins (UI test)
- [x] Clicking a role token changes every member without a pin (UI test)
- [x] The suggestion appears for two matching colonists and not again after "Not now"
- [x] Autotest screenshots of the board and the Roles lens

## 2026-09-26

No drag and drop in the UI engine, so moving a colonist between roles is pick up and put down: click their chip, then 'Put X here' on another card. The colonist menu also has Role rows. The why panel's role select waits for the why panel itself; the context menu covers it meanwhile. Rows don't wrap, so member chips come seven to a line.

## 2026-09-26

The screenshots of the board and the Roles lens come from the UI shots test (work_board_4_roles.png), not the client autotest sweep.
