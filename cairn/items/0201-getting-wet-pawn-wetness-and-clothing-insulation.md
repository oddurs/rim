---
id: 308074c6-261d-4dbc-9ed9-5761053703e7
title: 'Getting wet: pawn wetness and clothing insulation'
type: feature
status: backlog
milestone: crafting
depends_on:
- 03b9b791-082e-48f1-b51e-af6f42627685
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
