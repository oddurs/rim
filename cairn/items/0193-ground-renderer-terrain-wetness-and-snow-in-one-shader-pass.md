---
id: 193
title: 'Ground renderer: terrain, wetness and snow in one shader pass'
type: perf
status: backlog
milestone: scale
depends_on:
- 56
- 187
created: 2026-09-23
updated: 2026-09-23
priority: p2
api: none
effort: m
layer: client
area: render
pillar:
- performance
---

## Why

Terrain is drawn today as one rectangle per visible cell, every frame. Weather adds wetness, puddles and snow on every cell, which would double the draw calls. Drawing the ground from textures in one pass pays for the weather visuals and is the first step of 0098.

## What

- Map-sized textures: terrain colour (rebuilt on terrain change), ground state (R = wetness, G = snow; rows re-uploaded when their stock update runs).
- One full-screen quad with a shader: terrain colour with per-cell variation, darkened when wet, puddle sheen above 90% on low-drainage ground, snow cover blending in by depth.
- The old per-cell path is kept behind a flag until screenshots match, then deleted.

## Acceptance criteria

- [ ] Ground draws in one call; frame time at 4K fully zoomed out measured before and after
- [ ] Wet ground, puddles and snow visible in autotest screenshots (reviewed)
- [ ] Works on macOS and Linux CI (screenshots)

## 2026-09-23

Moved to Scale, next to 0098 (terrain render caching), which depends on it. It becomes worth doing when there is per-cell ground state to draw (0187).
