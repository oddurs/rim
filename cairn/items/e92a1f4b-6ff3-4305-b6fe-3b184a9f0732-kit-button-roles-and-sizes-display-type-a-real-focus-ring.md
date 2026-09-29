---
id: e92a1f4b-6ff3-4305-b6fe-3b184a9f0732
title: 'Kit: button roles and sizes, display type, a real focus ring'
type: feature
status: doing
milestone: rimos
assignee: Oddur Sigurdsson
claimed: 2026-09-28
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: additive
effort: s
layer: core
area: ui
---

## Problem

`kit.button` has one look plus `active`, so a window's main action looks like every other. There is no danger style. Focus only recolours the border, and the type scale stops at 18.

## Proposal

- `kit.button{…, kind = "primary" | "quiet" | "danger", size = "s" | "l"}`. The default stays today's secondary, medium.
- Text tokens `display = 22` and `hero = 28`.
- A focus ring: 2 px of accent, 1 px outside, on every focusable.

## Acceptance criteria

- [ ] The kit gallery shows every role and size in every state
- [ ] ui-shots before and after, in the PR; nothing existing changes look except the focus ring
- [x] rim_ui tests cover the four roles' tokens

## 2026-09-28

The focus ring is the engine's, not the kit's: it has to reach every focusable (inputs, menus, any mod's control), and it shows only for keyboard focus (Tab sets it, any press clears it), so a click never leaves a ring behind. The old per-button focus border is gone with it. Hover on an active button keeps its line_strong edge, as before.
