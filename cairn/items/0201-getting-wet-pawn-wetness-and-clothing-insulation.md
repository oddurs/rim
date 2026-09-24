---
id: 201
title: 'Getting wet: pawn wetness and clothing insulation'
type: feature
status: backlog
milestone: crafting
depends_on:
- 189
created: 2026-09-23
updated: 2026-09-23
priority: p2
api: additive
effort: m
layer: engine
area: needs
pillar:
- survival
---

## Why

Feels-like temperature (0189) penalises rain while it falls. A colonist who walked through a storm should stay cold until they dry off by a fire, and clothing should matter.

## What

- A pawn wetness state: rises under precipitation outdoors, falls indoors and faster near heat.
- Pawn-local modifiers on the warmth need's comfort range from wetness and apparel insulation (a stat pipeline input).

## Acceptance criteria

- [ ] Wet colonists feel colder until dry
- [ ] Apparel shifts the comfort range
