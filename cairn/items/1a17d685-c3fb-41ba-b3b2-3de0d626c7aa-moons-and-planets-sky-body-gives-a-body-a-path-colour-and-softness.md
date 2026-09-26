---
id: 1a17d685-c3fb-41ba-b3b2-3de0d626c7aa
title: 'Moons and planets: [[sky_body]] gives a body a path, colour and softness'
type: feature
status: backlog
milestone: lighting
depends_on:
- 8f4f1de8-5784-4377-8cee-25bcf223275e
- ebb814ad-369d-4ddd-99de-e755c2b14eb9
created: 2026-09-26
updated: 2026-09-26
priority: p2
api: additive
effort: m
layer: client
area: render
---

## Why

"A mod can add a second sun and a green moon" (DESIGN.md §4c) only matters if they light the world: their own colour, their own shadows, crossing the sky on their own path. 0209 (ebb814ad) gives them brightness cycles in the sim; this gives the renderer the rest. DESIGN.md §6e.

## What

- `[[sky_body]]` in defs: `term` (the labelled daylight term that sets its brightness), `rise`, `set`, `peak_elevation`, `arc`, `colour`, `angular_size`, `shadows`.
- Core declares the sun and a moon whose brightness follows 0209's cycle (phases).
- The shadow budget goes to the brightest bodies each rebuild.
- `docs/` example: a green-moon mod.

## Acceptance criteria

- [ ] A test mod adding a green moon lights the night green and casts its own shadows, with no engine or client change (autotest screenshot)
- [ ] With one shadow slot, the sun casts by day and the moon by night
