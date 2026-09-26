---
id: e5d8b445-42ed-4e87-8390-ad54d525757b
title: 'Occluders: one texel per cell of height, roof and opening'
type: feature
status: backlog
milestone: lighting
depends_on:
- 6a6dfe88-6d54-49a5-b871-f7eb78f69176
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: none
effort: m
layer: client
area: render
---

## Why

Every shadow in §6e is a march through one small texture. It has to say, per cell, how tall the cell is, whether it is roofed, whether it is a window or a door, and how much it stops sky and flame. DESIGN.md §6e.

## What

- An RGBA8 texture, one texel per cell: R height, G roofed / window / door, B sky opacity, A light opacity. Walls, doors and windows from their defs; trees and canopy from a `height` on the thing def (default by kind); solid terrain and today's rock things alike, so rock-is-terrain (8cc6252d) swaps the source without touching the shader.
- "Roofed" goes through one query. Today it is `map.indoors`; the roof span (24100bb9) replaces it with the per-cell `roofed(z, c)`.
- Rebuilt only when the map's wall, terrain or room revision changes, keyed like `Sky::update_lightmap` today. Once chunks (96d2dac9, on the grid branch) land, only dirty chunks are rewritten.

## Acceptance criteria

- [ ] A frame with no map change does no occluder work (test on the cache key)
- [ ] Rebuild time on 192 × 192 and 250 × 250 recorded here
- [ ] A window, a door and a tree read back with the expected channels (unit test on the packer)
