---
id: 5c9e649d-13df-4388-bb58-7c3f94cf90d8
title: Named saves, a quick save, and loading from inside a colony
type: feature
status: backlog
milestone: rimos
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: additive
effort: m
layer: client
area: save
---

## Problem

Saving is automatic only; loading happens only from the title or the command line.

## Proposal

- Save in the system menu makes a named save beside the autosaves, and Ctrl S makes a quick one; a toast confirms both.
- Colonies in a colony loads, saving the running colony first.
- Delete asks through the confirm dialog.

## Acceptance criteria

- [ ] A named save loads back to the same state (the save tests' hash)
- [ ] Loading from a colony saves it first, and the title lists both
