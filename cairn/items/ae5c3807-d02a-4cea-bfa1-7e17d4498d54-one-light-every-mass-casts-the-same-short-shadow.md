---
id: ae5c3807-d02a-4cea-bfa1-7e17d4498d54
title: 'One light: every mass casts the same short shadow'
type: feature
status: backlog
milestone: houses
depends_on:
- 3fe8c3cb-dcba-4882-b623-0468ea9fe697
created: 2026-09-26
updated: 2026-09-26
priority: p2
api: none
effort: s
layer: client
area: render
---

## Why

Top-down plans are flat. One offset shadow and a lit top edge give walls height without perspective, and they tell a blocking mass from a floor at a glance. DESIGN.md §6c.

## What

- Masses, pillars, fence rails and trees cast an offset shadow down and to the right, drawn before any mass so no shadow falls on a wall top.
- Exposed top and left contour edges get a highlight.
- Shadows sit under the lightmap, so night dims them with everything else.

## Acceptance criteria

- [ ] Shadows are baked into the chunk mesh; nothing is added per frame
- [ ] A wall's shadow never draws over a neighbouring wall (screenshot)
