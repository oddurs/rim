---
id: 371
uid: 7e45762c-0a2b-407f-b0fa-259bedb6cc34
title: 'Caverns: a plugin that opens −4, and what comes up from it'
type: content
status: backlog
milestone: world
depends_on:
- 330
- 333
created: 2026-09-26
updated: 2026-09-26
priority: p3
api: none
effort: m
layer: plugin
area: storyteller
pillar:
- plugin-first
---

## Why

The deepest level should be a place, not more rock: open pockets that were always there, and a second front for the storyteller. As a first-party plugin, it also tests that a mod can add a level (DESIGN.md §6d).

## What

- `mods/caverns` raises the level range to −4 and generates basalt with open caverns through `rim.on_generate_level`.
- Cavern creatures, and incidents that arrive through cavern openings once the colony breaks in. An opening is a new edge for arrivals.
- Rare materials that are only found there.

## Acceptance criteria

- [ ] Enabling the plugin on an existing save opens −4 without touching the levels above
- [ ] Breaking into a cavern enables cavern incidents; the storyteller sends one within a season (scene test)
- [ ] Core alone still plays, with no −4
