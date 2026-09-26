---
id: 12fbe8fb-c581-4f2b-81e6-bdd2a2eb72b3
title: Lighting presets, the settings file, and resolution that follows zoom
type: feature
status: backlog
milestone: lighting
depends_on:
- 6fd6b13b-1186-46f4-876e-743d173e03d7
- 8f4f1de8-5784-4377-8cee-25bcf223275e
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: none
effort: s
layer: client
area: render
---

## Why

Performance is the point. A player on the reference laptop must get a lit world inside budget without touching a setting, and a player with a better GPU should be able to ask for more. DESIGN.md §6e.

## What

- `low`, `medium` (default), `high`, `ultra`, `auto`: texels per cell, sky steps, bodies with shadows, soft shadows, contact shadows, dynamic cap and rays, bounce blur, upsampling, sky rebuild threshold. Values as in the concept's table.
- `[lighting]` in the player's settings file: `quality` plus any per-setting override.
- `auto` times the lighting passes for 120 frames on first launch and drops a preset while they exceed 1 ms.
- A light texel is never smaller than 4 screen pixels: zooming out lowers texels per cell.

## Acceptance criteria

- [ ] Presets and overrides round-trip through the settings file (test)
- [ ] At the minimum zoom the light buffer is 1 texel per cell (test)
- [ ] Bench numbers for each preset on the dusk scene recorded here

## 2026-09-26

No longer waits on moving lights (5a69f9c9, blocked on fire). The dynamic cap and rays settings arrive with that item; this one covers the rest.
