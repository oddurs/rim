---
id: 92f2ceaf-24c7-45ab-8401-bab550adceaa
title: 'Weather audio: rain, wind and thunder'
type: feature
status: backlog
milestone: '1.0'
depends_on:
- 98a2cd63-6776-428f-b4f8-d10d0708cacc
created: 2026-09-23
updated: 2026-09-23
priority: p2
api: additive
effort: m
layer: client
area: audio
pillar:
- survival
---

## Why

Rain on the roof and wind round the walls do as much for atmosphere as particles do. There's no audio system yet; weather is a good first user.

## What

- Looped ambience driven by precipitation, wind and whether the camera looks mostly at enclosed rooms; thunder timed to lightning flashes.
- Sounds declared in regime `visuals` (data), so mods add their own.

## Acceptance criteria

- [ ] Ambience follows the weather smoothly
- [ ] Sounds are data
