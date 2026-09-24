---
id: 297469a6-dae6-4334-a41f-6f0d5c80f7e5
title: 'Pawn AI: staggered thinking and job state machines'
type: feature
status: done
milestone: castaway
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: none
effort: l
layer: engine
area: ai
pillar:
- performance
---

## Why

Pawns only think when idle, on a staggered cadence, and run jobs as small state machines.

## Acceptance criteria

- [x] Colonist, hostile and animal thinkers
- [x] Reservations prevent two pawns taking one target
- [x] Interrupted jobs drop carried items
