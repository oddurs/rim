---
id: 194
title: Precipitation, fog and lightning
type: feature
status: planned
milestone: weather
depends_on:
- 56
- 184
- 188
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

Rain you can see is the difference between a number and a storm.

## What

- A client particle system driven by the regime's `visuals` and the precipitation, wind and temperature ambients: rain streaks slanted by wind, snow drifting and fluttering, sleet between. Density follows precipitation; nothing falls in enclosed rooms.
- Particle looks are data: `[[particles]]` in `mods/core/defs/visuals.toml` (count per screen, speed, length, colour, drift).
- Fog: a slow, scrolling low-contrast layer with a density curve. Lightning: storm-only flashes that briefly light the lightmap.
- Wind sways trees and bushes by wind speed.
- Capped at 3,000 particles, pooled, no allocation per frame.

## Acceptance criteria

- [ ] Rain, snow, fog and lightning each appear in autotest screenshots (reviewed)
- [ ] No particles inside enclosed rooms
- [ ] At most 0.3 ms per frame at 3,000 particles, recorded here
- [ ] A mod adds a particle look in data only
