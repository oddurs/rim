---
id: 8f4f1de8-5784-4377-8cee-25bcf223275e
title: 'Sun shadows: march the height map toward the sun, cached until it moves'
type: feature
status: backlog
milestone: lighting
depends_on:
- 0779def9-134c-4e87-abdf-2e3472fb2801
- 6a6dfe88-6d54-49a5-b871-f7eb78f69176
- e5d8b445-42ed-4e87-8390-ad54d525757b
created: 2026-09-26
updated: 2026-09-26
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

## Acceptance criteria

- [ ] A 1-cell wall's shadow at elevation 12° is 5 ± 0.5 cells long (autotest screenshot, or a readback test on the sky target)
- [ ] With time paused, frames run no sky pass (counter test)
- [ ] Bench: sky rebuild and steady frame recorded here, inside §8 on the dusk scene
- [ ] The old lightmap texture and its shader are gone
