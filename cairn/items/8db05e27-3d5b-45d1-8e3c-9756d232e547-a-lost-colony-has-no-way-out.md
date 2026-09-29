---
id: 8db05e27-3d5b-45d1-8e3c-9756d232e547
title: A lost colony has no way out
type: bug
status: backlog
milestone: rimos
depends_on:
- eea57f29-34f1-4530-af43-22b3749dde3a
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
effort: s
layer: core
area: ui
---

## Problem

When the colony is lost, core:lost (hud.luau) shows "The colony is lost / It lasted N days" as a modal with no buttons. The player can only close the game.

## Proposal

The modal gets Load a save (opens Colonies) and Leave to title; Esc does neither, so a stray key doesn't dismiss the news.

## Acceptance criteria

- [ ] From a lost colony, Leave to title reaches the title screen (autotest; fails today)
- [ ] Load a save opens the saves list
