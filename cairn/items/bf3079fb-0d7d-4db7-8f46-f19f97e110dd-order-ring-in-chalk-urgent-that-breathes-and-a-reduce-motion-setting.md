---
id: bf3079fb-0d7d-4db7-8f46-f19f97e110dd
title: Order ring in chalk, urgent that breathes, and a reduce-motion setting
type: feature
status: planned
milestone: chalkline
depends_on:
- d83192ed-0a3e-4a6f-a39a-04ce94a68935
created: 2026-09-27
updated: 2026-09-27
priority: p3
api: additive
effort: s
layer: client
area: ui
---

## Why

`order_flash` is a blue ring that shrinks from 1.4 cells, which reads as the player's colour rather than as acknowledgement. Urgent marks don't draw the eye. And nothing lets a player turn motion off. DESIGN.md §6f, "motion reports a change".

## What

- The order ring becomes one chalk ring on a keyline, growing from 0.3 to 0.75 cell over 240 ms.
- An urgent mark gets a ring that breathes out from it every 1.8 s, in `bad` at 35% fading to 0.
- A `reduce_motion` setting in the settings file, written like `render_scale`, with palette bindings "Reduce motion: on" and "Reduce motion: off". When it's on, hover, selection, the grid and the ring are instant, and the urgent ring is still.

## Acceptance criteria

- [ ] A unit test parses `reduce_motion = true`, and a bad value is an error, like `saved_render_scale`
- [ ] Autotest: after the "Reduce motion: on" binding, the urgent mark's ring radius is the same in two frames 0.5 s apart
- [ ] Screenshot `chalk-urgent`

## 2026-09-27

People (5d09b04e): gaits (809e1fc5) read this item's reduce_motion setting: every channel rests and pawns glide.
