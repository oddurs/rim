---
id: 5f022ef4-fb0d-41ab-8b55-1b470f6366f9
title: 'Defences count in the raid budget: walls, doors and trenches'
type: feature
status: backlog
milestone: defense
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: additive
effort: m
layer: core
area: storyteller
---

## Why

DESIGN.md §6d rules that trenches count as defence in the raid budget, like walls. Today nothing does: `rim.colony_strength` is the colonists' melee alone, so a walled or trenched colony draws the raids an open one does. Found while doing ba8253df.

## What

- A defence reading the storyteller can weigh: owned wall, door and drawbridge cells on the colony's perimeter, and trench cells, each by what it costs a raider to get through (hp, or bridge work).
- `threat_points` weighs it with strength.

## Acceptance criteria

- [ ] The balance harness's `--trench` and walled runs draw raids sized for their defence, recorded here
- [ ] Determinism test passes
