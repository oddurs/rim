---
id: 8b2e05db-9ea0-4d67-b62b-13fb8c9f4ae2
title: 'Work choice: an urgency term, so shelter beats nearby grass'
type: feature
status: backlog
milestone: colony
created: 2026-09-25
updated: 2026-09-25
priority: p2
api: none
effort: m
layer: engine
area: ai
---

## Why

DESIGN §4d says that within a priority level "an integer score picks: distance, urgency, skill fit". `find_work` scores distance only. The stone-age sweep (4bd94457) measured the cost: at the default priorities, a lone colonist spent 8,000 of day one's 11,667 ticks gathering grass beside them, while the hut that night needed went unbuilt. A player can answer it by setting Build to 1, but the default colonist should know that a planned shelter before dark beats another armful of fibre.

## What

- An urgency term in `find_work`'s score within a level. Sources: a blueprint for an enclosing wall or a bed with night coming and no shelter, food with hunger low, a threat in the colony.
- The sweep (`examples/stone_age.rs`) run at default priorities as the measure.

## Acceptance criteria

- [ ] At default priorities, the sweep's shelter-before-the-first-night share is within 10 points of the build-first bot's
- [ ] Nothing walks across the map for one pebble: distance still dominates between equally urgent jobs
