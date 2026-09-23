---
id: 29
title: Command queue applied at tick boundaries
type: feature
status: done
milestone: foundations
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: none
effort: s
layer: engine
area: sim
pillar:
- determinism
---

## Why

Every player action goes through a Command so replays and lockstep co-op are possible.

## Acceptance criteria

- [x] Designate, Build, Cancel, Draft, Move, Attack
- [x] UI never mutates the world directly
