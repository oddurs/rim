---
id: 86dcd0ca-9cde-459b-9b14-5f056f3d38e4
title: 'Several selected: box and shift selection with shared actions'
type: feature
status: done
milestone: interface
assignee: Oddur Sigurdsson
depends_on:
- 668c7762-97c3-4b65-bf33-55bca0ea9e66
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p2
api: additive
effort: m
layer: client
area: ui
---

## Why

Only one thing can be selected. Drafting a squad or checking several colonists means clicking each in turn.

## What

- The client keeps a selection set: shift-click adds or removes, a drag with the select tool selects the colonists in the box.
- `view.selection()` returns the set; the inspector shows a group summary and the actions every member shares (drafting the lot is one click).
- Commands still go one per pawn through `Command`, so the sim stays deterministic.

## Acceptance criteria

- [x] Shift-click and box select build a selection
- [x] The inspector shows a group summary for more than one
- [x] Draft applies to every selected colonist

## 2026-09-26

Client keeps selected (primary) + group. Select acts on release: a click under 6 px picks, a drag boxes colonists. Group actions opt in with group = true; core:draft drafts all unless all are. Right-click orders each selected pawn the order resolves for. Autotest section runs last because it adds colonists.
