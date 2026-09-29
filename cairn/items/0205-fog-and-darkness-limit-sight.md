---
id: 205
uid: 713009ac-05a0-4ad0-af84-1e50001dc8f5
title: Fog and darkness limit sight
type: feature
status: backlog
milestone: defense
depends_on:
- 56
- 184
created: 2026-09-23
updated: 2026-09-26
priority: p3
api: additive
effort: m
layer: engine
area: combat
pillar:
- survival
---

## Why

Night should feel dangerous (0056). Letting light and fog shorten sight makes night raids and fog banks tactical, not just visual.

## What

- Sight range from the light field and fog density; detection and aggro use it.
- Predators prefer to hunt at night.

## Acceptance criteria

- [ ] Hostiles are spotted later at night and in fog (test)
- [ ] Campfires and lights extend sight around them

## 2026-09-26

DESIGN.md §6e rules that the rendered shadows are cosmetic and the sim's light field stays the authority. Where they disagree (windows, doors, tree shade), anything the player uses to judge sight must show the field value, not the picture.
