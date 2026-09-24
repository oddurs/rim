---
id: 217
title: 'Windows: light in, heat out'
type: content
status: backlog
milestone: building
depends_on:
- 216
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: none
effort: s
layer: core
area: building
---

## Why

The first boundary piece whose whole purpose is that it is not a wall.

## What

- A `window` def: blocks movement, bounds the room, and contributes
  daylight in and a large leak out.
- Built of the same materials as everything else, plus glass; a metal frame
  leaks differently from a wooden one, which falls out of 0213 and 0216
  rather than being written specially.

## Acceptance criteria

- [ ] A room with a window is lit by day without a lamp
- [ ] It loses heat faster than the same room without one
- [ ] Raiders cannot walk through it, and it can be broken like a door
