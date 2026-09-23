---
id: 169
title: 'Anchored layer: world-attached labels, bars and bubbles without overlap'
type: feature
status: planned
milestone: interface
depends_on:
- 168
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: none
effort: m
layer: client
area: ui
pillar:
- performance
---

## Why

Names, health bars, the sleep marker, the order label and speech bubbles belong to things in the world, and today they overlap.

## What

Anchored nodes attach to an entity or a cell and follow the camera. A placement pass nudges overlapping labels apart by priority (selected, colonists, hostiles, others) and culls offscreen ones. Bubbles have a lifetime and a tail pointing at their anchor.

## Acceptance criteria

- [ ] Two adjacent pawns' names never overlap (test with a crowd of 10)
- [ ] Priority order: selected, colonists, hostiles, others
- [ ] Offscreen anchors cost nothing
- [ ] A bubble component with lifetime and tail; core shows one when a colonist joins
- [ ] 50 anchored labels under 0.2 ms per frame
