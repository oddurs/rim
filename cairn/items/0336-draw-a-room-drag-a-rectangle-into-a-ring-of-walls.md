---
id: 336
uid: 454f4bde-ee82-44e8-8875-b5fc66739591
title: 'Draw a room: drag a rectangle into a ring of walls'
type: feature
status: done
milestone: houses
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-26
priority: p2
api: none
effort: s
layer: client
area: ui
---

## Why

Most walls are built as rings. Prison Architect draws a room with one drag, and the prototype's room tool shows how little it needs.

## What

- A build mode: drag a rectangle, and a ring of wall plans follows the cursor. Release to place it, skipping doors and windows already there.
- It uses the material picked in the tray.

## Acceptance criteria

- [x] One drag places a closed ring (autotest)
- [x] Dragging over an existing door keeps the door

## 2026-09-26

The tool already existed: a drag with a blocking build has placed a ring since Castaway (71b3bd4, build_rects), with the cells shown as it follows the cursor, and the autotest already checks the 20-cell outline. Existing fixtures are skipped because spawn_fixture_of refuses an occupied cell. What was missing was a check that a door survives a ring drawn over it; added to the autotest.

## 2026-09-27

The new check failed at first and found an older fault: the autotest clicked build:door and build:bed, which aren't tool keys (they're build:core:door and build:core:bed), so both clicks did nothing and walls went where the door and bed were meant to. The plan count still came to 21, so nobody noticed. Fixed the keys, and each click now checks its tool was selected.
