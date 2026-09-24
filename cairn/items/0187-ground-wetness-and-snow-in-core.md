---
id: 187
title: Ground wetness and snow in core
type: content
status: planned
milestone: weather
depends_on:
- 184
- 186
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: none
effort: m
layer: core
area: map
pillar:
- survival
- growth
---

## Why

The ground is where weather lasts. Wetness drives plant growth and farming; snow is winter you can see and wade through.

## What

- `wetness` (0–100%): rain soaks in under open sky; drying toward the water table, faster when warm, dry and windy; water tiles pinned at 100; snowmelt feeds it; `near = "water"` raises the base.
- `snow` (0–60 cm): precipitation below 0°C accumulates (curve over temperature for sleet), melts above 0°C by degree-hours, faster in rain.
- Both under open sky only; enclosed rooms stay dry.
- Tuning targets: a day of rain takes grass from its base to about 80%; it dries back in about two sunny days; sand dries in half a day; marsh never drops below 70%; a winter week of snow gives 20–40 cm; spring melts it within four days.

## Acceptance criteria

- [ ] Wetness and snow defined in `mods/core/defs/fields.toml` only
- [ ] Tuning targets met on the climate report (numbers recorded here)
- [ ] Enclosed rooms stay dry and snow-free
- [ ] Overlay and hover show both
