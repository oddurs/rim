---
id: 0e1d71a9-fe0d-475b-9e0f-90e259a31b37
title: Esc steps back in one order, ending in a system menu
type: feature
status: backlog
milestone: rimos
depends_on:
- 2adfa3ac-b19d-41b3-bbfb-eeb18b7f3191
- eea57f29-34f1-4530-af43-22b3749dde3a
created: 2026-09-28
updated: 2026-09-28
priority: p0
api: none
effort: m
layer: core
area: ui
---

## Problem

Esc closes a menu, unfocuses a field, drops the tool, closes a tray or a sheet, then clears the selection. It never closes ordinary windows, and there is no pause menu. (The palette's own Esc leak is fixed separately by 2c567482.)

## Proposal

One order:
1. menu or popup;
2. the launcher;
3. a dialog's safe answer;
4. the tool, then the tray;
5. the focused app, if it closes on Esc;
6. the selection;
7. the system menu: Resume, Save, Colonies, Settings, Leave to title, Quit.

Leaving and quitting save first, so they don't confirm.

## Acceptance criteria

- [ ] Autotest walks the whole order from a state with every layer open
- [ ] The system menu pauses the game and resumes it on close
