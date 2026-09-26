---
id: 454f4bde-ee82-44e8-8875-b5fc66739591
title: 'Draw a room: drag a rectangle into a ring of walls'
type: feature
status: backlog
milestone: houses
created: 2026-09-26
updated: 2026-09-26
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

- [ ] One drag places a closed ring (autotest)
- [ ] Dragging over an existing door keeps the door
