---
id: 24edebdb-360f-41fe-b0c5-005d01b81fba
title: The autotest clicks a tray row the list has scrolled out of view
type: bug
status: done
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p0
api: none
effort: s
layer: client
area: tests
---

## Why

Main went red on four autotest checks after mining (#298) gave the Orders tray a ninth row. The tray is a fixed height and its list scrolls, as designed; but click_tool clicked the Cancel row's layout rect, which is where it would be unscrolled, below the tray's bottom, and hit nothing. The other three failures followed from Cancel never being picked. Green-forest found the cause; its fix was left uncommitted.

## What

- The tray's list is `core:dock.list`, so a test can read how far it is scrolled.
- click_tool wheels the row into view and clicks where it is drawn.

## Acceptance criteria

- [x] The autotest is green on main with nine Orders rows
