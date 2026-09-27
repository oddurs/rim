---
id: bea51754-c7d0-4353-b763-1a6a11e6f383
title: 'Premises: [[start]] becomes a choice, with a cast, history and an opening'
type: feature
status: backlog
milestone: story
depends_on:
- 39915ec5-4ff9-40a7-af6b-aa6d5dbd39a0
- 801c8f78-76af-4aa0-9394-23388a7d1d3c
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: l
layer: core
area: sim
pillar:
- growth
- plugin-first
---

## Why

A premise sets how a run opens and never its difficulty (DESIGN.md §4g).
Today there is one `[[start]]` def and a hardcoded opening message.

## What

Mods add premises; a new game picks one, and core's castaway is the default.
Fields: map constraints, cast (name, traits, skills, founder), carry, history
(relation reasons between cast members), secrets, an opening text key, and an
optional generator script. No field touches the raid budget, wealth or eras.

## Acceptance criteria

- [ ] Several `[[start]]` premises load; the castaway is the default
- [ ] Cast, carry, history and opening applied at game start
- [ ] A script hook at game start, before tick 0 events
- [ ] Map constraints: biome now; features once the map-generation hook lands
- [ ] A game with no story mod plays exactly as before (determinism test unchanged)
