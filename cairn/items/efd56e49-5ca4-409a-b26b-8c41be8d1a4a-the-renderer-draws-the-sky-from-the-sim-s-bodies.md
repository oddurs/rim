---
id: efd56e49-5ca4-409a-b26b-8c41be8d1a4a
title: The renderer draws the sky from the sim's bodies
type: feature
status: backlog
milestone: lighting
depends_on:
- 0a27bfbb-276b-4b28-a7cc-1e02a3f8dc91
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: breaking
effort: m
layer: client
area: render
---

## Why

For the picture and the sim to be one model, the renderer should take each body's position and brightness from the sim (0a27bfbb) and keep no curves of its own.

## What

- The client reads each body's `altitude`, `azimuth`, `up` and `phase` from the sim's accessor. Its own `rise`/`set`/`peak`/`arc` go. Shadows follow the real azimuth, so they swing through the seasons.
- The moon's disc and halo show its phase. Its light in the picture comes from the same number the sim uses.
- Removing `rise`/`set`/`peak`/`arc` from `[[sky_body]]` is a defs break: bump the API version as DESIGN.md says, and update the two-suns example and the docs.
- Lighting presets, the autotest's lighting sections and the render bench keep working.

## Acceptance criteria

- [ ] The client reads no rise/set/peak/arc; the fields are gone from the schema and the API version is bumped
- [ ] At the same hour a shadow points differently at midsummer and midwinter (autotest)
- [ ] The picture's moonlight and the sim's are one number (test or autotest)
- [ ] The render bench stays within budget in the queue lane
