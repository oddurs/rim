---
id: 440
uid: ff818bb3-7175-48f0-a3c3-c346c9bc2469
title: Lightning casts shadows
type: feature
status: done
milestone: lighting
assignee: Oddur Sigurdsson
depends_on:
- 379
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
priority: p3
api: none
effort: s
layer: client
area: render
---

## Why

sky.rs already flashes the screen in heavy storms. A flash from a direction, throwing every wall's shadow across the ground for a frame, is the most dramatic thing lighting can do and costs one sky rebuild. DESIGN.md §6e.

## What

- A flash is a sky body for its few frames: high elevation, random azimuth, blue-white, hard shadows, full exposure.

## Acceptance criteria

- [x] A flash rebuilds the sky target at most twice (on and off) (counter test)

## 2026-09-27

Done as: while a flash is brighter than 0.1, the sun pass's key is the flash's (40 deg up, its own azimuth, soft 0), so the pass runs once as it strikes and once as it fades back to the sun's key. The multiply's bolt term, FLASH_RGB x strength x 0.65, lands where sunlit reaches, and 0.35 of the flash lights everything via the sky. On the frame a flash starts, before prepare has seen it, it lights evenly as it did before. Sky::flash() returns strength and azimuth, with a random azimuth each flash. Autotest: at night a strike lights some texels (>0.9) while walls shade others, and the whole flash took 2 sun passes. Found on the way: the lighting checks after the storm section were at the mercy of its flashes, and the eye's exposure, adapted to the night, pushed the sun section's pinned day past white, which is why the contact-shadow check read 0.91 now and then. The autotest calms the weather through the lighting sections, and Light::adapt_now lets the eye settle at once (in 2f13e01d, where exposure arrived).

## 2026-09-27

Review fixes: while sunlit holds the bolt, the sun's direct share goes to the sky for those frames, rather than following the bolt's shadows. Roofs take a flash as the ground does (0.35 of it on the sky, the rest blue-white). The counter test stops the rain once it has its shot, so a natural flash can't strike while the first fades on a slow machine.
