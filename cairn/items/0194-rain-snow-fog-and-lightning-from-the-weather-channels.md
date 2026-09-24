---
id: 98a2cd63-6776-428f-b4f8-d10d0708cacc
title: Rain, snow, fog and lightning from the weather channels
type: feature
status: done
milestone: weather
depends_on:
- 2e0b9d43-90e5-4051-bda6-81544e9caf0f
- 7c50b502-5e27-4807-a36c-0654fe9aec97
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
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

- [x] Rain, snow, fog and lightning appear in autotest screenshots (reviewed)
- [x] No particles in enclosed rooms
- [x] At most 0.3 ms per frame at 3,000 particles, recorded here

## 2026-09-23

Particles from channels only: density from precipitation, snow below 0°C and a mix around freezing, slant and drift from wind/wind_dir, fog as a veil plus drifting banks, lightning in heavy precipitation with wind > 10 m/s (the flash brightens the lighting uniform). Screenshots reviewed: rain, snow, fog, storm at night. Nothing drawn over enclosed rooms (autotest: a walled hut, drops hidden over it counted > 0). Cost: 52 µs for 1,500 drops, 103 µs for 3,000 (CPU). Looks are constants in sky.rs for now; data with a later visuals pass.
