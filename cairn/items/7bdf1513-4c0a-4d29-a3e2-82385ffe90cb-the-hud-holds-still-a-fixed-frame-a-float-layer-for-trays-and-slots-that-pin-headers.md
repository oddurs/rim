---
id: 7bdf1513-4c0a-4d29-a3e2-82385ffe90cb
title: 'The HUD holds still: a fixed frame, a float layer for trays, and slots that pin headers'
type: bug
status: review
milestone: interface
assignee: Oddur Sigurdsson
claimed: 2026-09-26
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: additive
effort: m
layer: client
area: ui
---

## Why

Clicking in the game makes the bottom-left and bottom-right panels jump. The
bottom region is as tall as its contents, and three transient things live in
it: the undo toast (5 s after every order), the Build tray and the placing
pill. The inspector and the hover card sit on the band's floor, so each one
rides up and down with them, and the inspector also grows upward when its
content changes. Design: the Steady HUD artifact
(https://claude.ai/artifact/E4SyhBFP7gZFgWt762ZLw6).

## What

- The frame is fixed: the bottom region holds only the dock bar.
- A `float` layer lays panels out in the band's centre without reflowing the shell; the tray moves there.
- Status lives in the bar: the placing pill and undo move into the dock bar's status slot.
- `slot` on a mount reserves a minimum height at a column's end and pins the panel's header to its top.

## Acceptance criteria

- [x] `ui.mount("float", ...)` places panels in the band's centre, above docked panels, with no effect on the band
- [x] `slot = <px>` on a mount pins an end-aligned panel's header to the top of a reserved height
- [x] The tray floats; the pill and the undo toast sit in the dock bar
- [x] The inspector and the hover card mount with slots
- [x] An autotest plays select, order, build, pick, escape, tab and hover, and asserts the band and the docked headers don't move
- [x] DESIGN.md §11 records the rules

## 2026-09-26

Slots are 280 for the inspector (tallest tab, Work, measured 269) and 132 for the hover card. The float layer places panels from the left column's edge and slides one too wide for the room left to stay on screen (the tray is 876 wide, so at 1024 it covers part of the left column). The autotest's click_tool now waits a frame after a pick: input routes against the previous frame's layout, and a stale tray over the map ate the next press, which a person can't do within one frame.
