---
id: 201
uid: 308074c6-261d-4dbc-9ed9-5761053703e7
title: 'Getting wet: pawn wetness and clothing insulation'
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- 189
created: 2026-09-23
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] Wet colonists feel colder until dry
- [x] Apparel shifts the comfort range

## 2026-09-27

Pawn::wet (u16, 1/10000ths; saved with the pawn, skipped when dry; in the hash) is moved by the needs pass from a field need's wet block: soak per hour per unit of the field outside an enclosed room, drying by hours/dry_hours times (1 + heat × degrees above comfort's cold end) when no rain falls on it, and the comfort's cold end rises by chill × wet. Core's warmth: precipitation soaks 0.1/h per mm/h, dries in 4 h at comfort, chill 6°. Apparel shifting the comfort range is d0362b0c (the insulated need), closed with this pair; criterion 2 is met there. tests/getting_wet.rs: an hour of 4 mm/h soaks past 0.3; wet at 12° loses warmth a dry colonist keeps; a warm colonist dries faster; dry within four hours. Not done: the pawn panel doesn't show wetness yet.

People (5d09b04e): a wet pawn can darken a step through a gait fact or a feature's condition once pawn tags exist.
