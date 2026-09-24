---
id: 56
title: 'Lighting from the light field: lightmap, glow and sky tint'
type: feature
status: planned
milestone: weather
depends_on:
- 183
created: 2026-09-22
updated: 2026-09-23
priority: p0
api: additive
effort: m
layer: client
area: render
pillar:
- survival
- performance
---

## Why

Night should feel dangerous, and a fire should be a place. Today the client darkens the screen on its own hard-coded curve; the sim's `light` field (daylight, dark indoors, firelight) goes unused by the renderer.

## What

- The renderer lights the world from the `light` field: a lightmap texture of stamped light plus an indoor mask, with the ambient passed as a uniform, so the texture changes only when emitters or rooms change.
- A multiply pass over the whole world (terrain, things and pawns), after the ground pass.
- Light's ambient comes from daylight terms over the hour and year (sunrise and sunset move with the season), dimmed by cloud and storms.
- Sky tint over the day as a colour curve in data (dawn warm, dusk violet, night blue), and the regime can shift it.
- `darkness()` in draw.rs is deleted.

## Acceptance criteria

- [ ] Smooth light curve from the light field; the client's own curve is gone
- [ ] Campfires glow at night and enclosed rooms are dark without light (screenshots reviewed)
- [ ] Sky tint and storm darkening come from data
- [ ] Lighting costs at most 0.3 ms per frame at 4K, recorded here
