---
id: 8d8cc16f-b5a7-4da3-9308-3991eed8e94b
title: 'Windows: light in, heat out'
type: content
status: done
milestone: building
assignee: Oddur Sigurdsson
depends_on:
- 3f4c257d-5e6f-4417-86d4-58e1cc981c73
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

- [x] A room with a window is lit by day without a lamp
- [x] It loses heat faster than the same room without one
- [x] Raiders cannot walk through it, and it can be broken like a door

## 2026-09-23

Glass is not a material here, for the same reason metal was not in 0214: nothing produces it, so a 'glass' item would never appear in a game. The window is built of the frame's material and the pane is implied; a glass material arrives with whatever mod adds a source, and would want a 'pane' category rather than 'structural'.

## 2026-09-23

Done, as content plus one engine generalisation. Content: window blocks, bounds the room, pass 0.35 on light, leak 4.0 on temperature, hp 60, 3 of anything structural, its own Shape::Window. Engine: criterion 3 said 'broken like a door', but nearest_breach only ever targeted locked doors, so a window would have been unbreakable in practice. It now picks the weakest owned piece standing between the raider and its target (ties to nearest): window 60 before door 150 before wall 200. Side effect, deliberate and tested: a hut with no door or window now gets a wall dug through rather than a raider milling about outside forever. Natural rock is never a target (no Owner). Job::Breach { door } renamed to { target }. Glass: see the earlier note.
