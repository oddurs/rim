---
id: 376
uid: 8baaf318-6d58-4143-9880-617c1b012dfc
title: 'Pawns jitter as they walk: draw them at the fraction of a tick'
type: bug
status: done
milestone: scale
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p0
api: none
effort: s
layer: client
area: render
---

## Why

Pawns stutter as they walk. The sim steps at a fixed 60 ticks a second and a pawn is drawn where its last tick left it; frames don't line up with ticks, so at 60 Hz some frames run no tick and some run two, and at 120 Hz (ProMotion) every tick is shown for two frames. The name and speech labels follow the same stepped position.

## What

- The client keeps the fraction of a tick its accumulator holds between ticks, and every pawn (and every label anchored to one) is drawn that far into its current step.
- Paused, the fraction is zero; at 3x and 6x it is still the remainder of the frame's ticks.

## Acceptance criteria

- [x] A walking pawn's drawn position moves every frame, by an even amount at a steady speed
- [x] Names and speech bubbles move with it, not a tick behind
- [x] Nothing moves while paused

## 2026-09-26

Pawn::drawn_at(frac) is the one place a pawn's drawn position comes from; the client passes the accumulator's part-tick to the draw, picking, box select and anchored labels alike. Pause keeps the part-tick instead of zeroing it, so a paused pawn doesn't slide back.
