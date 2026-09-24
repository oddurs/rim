---
id: 203
title: 'Biomes: other climates as a plugin'
type: content
status: backlog
milestone: world
depends_on:
- 184
- 187
created: 2026-09-23
updated: 2026-09-23
priority: p2
api: none
effort: m
layer: plugin
area: map
pillar:
- plugin-first
- growth
---

## Why

Core ships one temperate climate (DESIGN.md §4c). Deserts, tundra and monsoon coasts are the obvious proof that climate is data: each is a patch to `[climate]`, a set of regimes and terrain bands.

## What

- A first-party `biomes` plugin: arid (dry heat, sandstorms, sparse growth), boreal (long winters, deep snow) and monsoon (a wet season).
- Biome choice at world creation; map generation bands per biome.

## Acceptance criteria

- [ ] Each biome is data plus at most a small script, with no engine change
- [ ] Climate report for each biome recorded
