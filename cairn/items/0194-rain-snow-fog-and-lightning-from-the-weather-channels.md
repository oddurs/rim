---
id: 194
title: Rain, snow, fog and lightning from the weather channels
type: feature
status: planned
milestone: weather
depends_on:
- 56
- 184
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: additive
effort: m
layer: client
area: render
pillar:
- survival
- performance
---

## Why

Rain you can see is the difference between a number and a storm. The renderer reacts to channels, not weather names, so any mod that sets `precipitation` gets rain.

## What

- Particles from `precipitation` (density), `temperature` (rain, sleet or snow) and `wind`/`wind_dir` (slant and drift). Nothing falls in enclosed rooms.
- Fog: a slow translucent layer from `fog`. Lightning: flashes in heavy precipitation with strong wind, lighting the lightmap.
- Pooled, capped at 3,000 particles, no allocation per frame. Looks are constants for now (data later, with an icon and visuals pass).

## Acceptance criteria

- [ ] Rain, snow, fog and lightning appear in autotest screenshots (reviewed)
- [ ] No particles in enclosed rooms
- [ ] At most 0.3 ms per frame at 3,000 particles, recorded here
