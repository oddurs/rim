---
id: 4766747b-db93-43d4-a49e-666b91ed6529
title: 'Frames are slow and the world blurry on a Retina Mac: draw the world offscreen, full resolution'
type: bug
status: done
milestone: graphics
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: none
effort: s
layer: client
area: render
---

## What happens

## What should happen

## Reproduction

Seed:
Mods:
Tick:

1.

On a 2x Mac the game felt low frame rate and low resolution. Two causes:
the world drew straight to the window, where each GL pass (the batch,
each chunk-mesh layer) cost the frame 13-33 ms to submit; and a
Retina screen defaulted to half the resolution to save pixels.

## Acceptance criteria

- [x] The world draws into its own target at every render scale
- [x] Unset render scale is full resolution
- [x] The render bench's submit time at full scale is under 5 ms on the whole map

## 2026-09-26

Bench at 2x, alternating runs under heavy load: direct 13-33 ms submit per frame, through a target 1.7-8 ms; after the change 2.1-4.4 ms at full scale. Mechanism not proven, but the difference is repeatable.
