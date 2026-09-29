---
id: 8cfce3bb-b22f-43ec-88f8-21bab9326787
title: A confirm dialog for what can't be undone
type: feature
status: doing
milestone: rimos
assignee: Oddur Sigurdsson
claimed: 2026-09-29
created: 2026-09-28
updated: 2026-09-29
priority: p1
api: additive
effort: s
layer: core
area: ui
---

## Problem

There is no confirm component, only one inline confirmation in the Work sheet. 

## Proposal

- `kit.confirm{title, body, verb, danger}`: the title is the question, the buttons say the verb, and the safe one has focus.
- Used for deleting a save, overwriting a named save and removing a role with members.

## Acceptance criteria

- [x] Esc and Enter on a confirm answer safely (test)

## 2026-09-29

PAUSED: done: kit.confirm (confirm.luau), modal roots take keys and outside presses (lib.rs holding_roots), Work's role delete uses it, tests/confirm.rs + board.rs pass. Left: docs/modding/ui.md (kit.confirm out of 'planned', modal on_key/on_outside), full rim_ui suite, gate. Next step: rebase onto main after e92a1f4b merges (branch is stacked on it), then docs. Branch feat/8cfce3bb-confirm, draft PR.
