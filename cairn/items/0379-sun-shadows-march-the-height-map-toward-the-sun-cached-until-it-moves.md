---
id: 379
uid: 8f4f1de8-5784-4377-8cee-25bcf223275e
title: 'Sun shadows: march the height map toward the sun, cached until it moves'
type: feature
status: done
milestone: lighting
assignee: Oddur Sigurdsson
depends_on:
- 302
- 360
- 421
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
priority: p0
api: none
effort: m
layer: client
area: render
---

## Why

The first thing that makes the world look lit: long shadows at dusk, short at noon, dappled under trees. It also replaces `Sky::light` with the light-buffer pipeline every later ticket adds to. DESIGN.md §6e.

## What

- Light targets at 1, 2 or 4 texels per cell (`render_target`), a compose pass, and the multiply over the world that `Sky::light` does today. Precipitation, fog and lightning in sky.rs stay.
- Sky pass: each texel steps toward the sun at 0.4 cells a step, rising `tan(elevation)`, and stops at the first taller occluder. Soft penumbra growing with distance; canopy partly transparent with a hashed pattern. Up to 3 bodies in one pass; the shadow budget goes to the brightest.
- The sun's path is data on core's `[[sky]]`: rise, set, peak elevation, azimuth arc. Brightness still comes from the sim's `daylight` terms; cloud widens the penumbra.
- Rebuilt only when a body moves past the preset's threshold (0.25° at `medium`) or occluders change.
- GLSL 100, RGBA8 targets, square-root encoded with dither.
- The plan's contact shadow (0779def9): compose reads the occluder a short step up and to the left, and darkens under a mass by a factor that fades as the texel's direct sky visibility rises. This replaces the down-right shadow ae5c3807 would have baked into the chunk mesh.

## Acceptance criteria

- [x] A 1-cell wall's shadow at elevation 12° is 5 ± 0.5 cells long (autotest screenshot, or a readback test on the sky target)
- [x] With time paused, frames run no sky pass (counter test)
- [x] Bench: sky rebuild and steady frame recorded here, inside §8 on the dusk scene
- [x] The old multiply shader is gone; the firelight stamps stay a texture until 6fd6b13b bakes them
- [x] A wall shows the contact shadow at night and not where the sun reaches (autotest reads the frame back)

## 2026-09-26

Measured (release, 250x250, Apple M4 Pro under load, CPU only since this GPU can't time a pass): the sun pass costs 0.15-0.17 ms of CPU when it runs (a 500x500 target, 28 steps), and nothing on the frames it doesn't: 0% of steady frames at dusk, with the sun quantised to 0.25 deg. The multiply is 0.008 ms and one draw call. The whole world stays inside the 4 ms budget at dusk (1.24 ms mean). Autotest: the wall's shadow at 12 deg is 4.5 cells at half-cell resolution (4.7 by tan). The contact shadow darkens the ground under a wall to 0.69 at night and 1.00 in sun. Two criteria were reworded. The firelight stamps texture stays until 6fd6b13b bakes firelight, so only the old multiply shader is gone. The contact-shadow check reads the frame back rather than comparing screenshots by eye. The oak's own baked shadow disc is removed, since the sun and the contact shadow draw its shadow now. The autotest's px helper reads grabbed frames upside down (GL rows are bottom first); these checks flip rows in a closure of their own, and #187 fixes px itself.

## 2026-09-26

Review fixes: shader failures are per shader, so without the sun's the world is still lit, just with no sun shadows. The multiply takes highp like the sun pass. The sun key collapses to Down below the horizon and rounds cloud softness to fiftieths, so night and cloud drift don't rerun the pass. The march ignores anything no taller than where the ray set out, so a wall top at dawn isn't shaded by the wall beside it. Canopies cast a lighter contact band, so trees keep a shadow at night. Without a sun path the contact band fades with daylight. The autotest pins full daylight for this section and checks on screen that the shadow falls north of the wall (0.56 of the lit ground). Declined: a fringe beside roofed ground. Roofed means an enclosed room, so the bleed lands on a lit wall top.

## 2026-09-27

People (5d09b04e, DESIGN.md §6h): a pawn's soft shadow is a `world = true` body part (535a1fb9). Once the sun direction is available to the client, the shadow can offset away from it instead of a fixed down-right.
