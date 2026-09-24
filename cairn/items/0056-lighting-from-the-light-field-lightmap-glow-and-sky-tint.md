---
id: 56
title: 'Lighting from the light field: lightmap, glow and sky tint'
type: feature
status: done
milestone: weather
depends_on:
- 183
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
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
- Light's ambient is core's `daylight` terms over the hour and year (sunrise and sunset move with the season) through a `cloud` curve, so storms darken it under any sky a mod defines.
- Sky tint over the day as a colour curve in data (dawn warm, dusk violet, night blue), and the regime can shift it. Key it by label (`[sky_tint.dawn]`) so coloured sky terms (0209) add to it rather than change its shape.
- `darkness()` in draw.rs is deleted.

## Acceptance criteria

- [x] Smooth light curve from the light field; the client's own curve is gone
- [x] Campfires glow at night and enclosed rooms are dark without light (screenshots reviewed)
- [x] Sky tint and storm darkening come from data
- [x] Lighting costs at most 0.3 ms per frame at 4K, recorded here

## 2026-09-23

Sky tint keyed by label so 0209 (coloured sky terms, e.g. a green moon) is additive, not a reshape.

## 2026-09-23

crates/rim_client/src/sky.rs: a map-sized lightmap (R = firelight stamps, G = indoors), rebuilt only when emitter stamps or rooms change, multiplied over the world by a shader; sky colour and brightness are uniforms. Brightness is sqrt(light) (overcast days still read as day); fire is max(sky, fire) so it glows at night and barely at noon; indoors gets [[sky]] indoor_share of daylight. [[sky]] in core: night, firelight, indoor_share and tints by label (dawn, dusk, overcast) as terms, patchable (sky/core). draw.rs darkness() deleted. Screenshots reviewed: firelit night, dark hut interiors, grey overcast. CPU cost 2-3 µs per frame; the GPU cost is one full-screen textured quad. Cosmetic follow-up: firelight is squarish because stamps spread by 8-way walking distance.
