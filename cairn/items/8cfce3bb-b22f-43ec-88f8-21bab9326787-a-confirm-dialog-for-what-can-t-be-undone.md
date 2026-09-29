---
id: 8cfce3bb-b22f-43ec-88f8-21bab9326787
title: A confirm dialog for what can't be undone
type: feature
status: backlog
milestone: rimos
created: 2026-09-28
updated: 2026-09-28
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

- [ ] Esc and Enter on a confirm answer safely (test)
