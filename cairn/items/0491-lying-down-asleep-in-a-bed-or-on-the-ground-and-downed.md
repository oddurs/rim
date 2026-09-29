---
id: 491
uid: 6cb4f2f9-4a5c-4b13-a2bb-a1c5377ebb09
title: 'Lying down: asleep in a bed or on the ground, and downed'
type: feature
status: backlog
milestone: people
depends_on:
- 480
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: additive
pillar:
- performance
effort: s
layer: client
area: render
---

## Why

A sleeping colonist and a downed one are drawn standing. DESIGN.md §6h gives
each a lying pose, and a body left by death (66617791) reuses it.

## What

- **Asleep in a bed:** lying along the bed, the head on its pillow end
  (the bed's facing, docs/modding/looks.md "Facing"), under the bed's
  blanket layers, which draw over the sleeper.
- **Asleep on the ground:** lying along the last facing, no blanket.
- **Downed:** lying where they fell, limbs apart, at an angle from the
  entity id so a group of downed raiders doesn't look stamped.
- A `lie` pose per body: part offsets for lying, so animals lie too.
- The sleep marker stays anchored UI (core:labels).

## Acceptance criteria

- [ ] A colonist asleep in a bed draws lying under the blanket, head at the pillow, for all four bed facings (autotest screenshots)
- [ ] Asleep outdoors and downed poses draw (screenshots)
- [ ] Downed angles vary by entity and are stable across frames and replays (test)
- [ ] `lie` is documented in docs/modding/bodies.md
