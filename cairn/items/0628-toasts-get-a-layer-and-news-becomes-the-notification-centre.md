---
id: 628
uid: 6a6b9fcf-edfd-4f57-b68d-f8906618befa
title: Toasts get a layer, and News becomes the notification centre
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

`kit.toast` exists but has no layer; the undo chip is the only toast-like thing. Saving and settings changes need somewhere to speak.

## Proposal

- A toast layer, bottom right above the dock: at most three, four seconds each, paused while hovered. `ui.toast(text, kind)` for mods.
- The undo chip becomes a toast with an action.
- News gains filters by kind.

## Acceptance criteria

- [ ] A fourth toast drops the oldest; hovering keeps one up (test)
- [ ] Deconstruct's undo still works from the toast (autotest)
