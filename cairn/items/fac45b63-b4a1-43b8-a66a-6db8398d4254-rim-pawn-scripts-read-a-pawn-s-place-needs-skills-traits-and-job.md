---
id: fac45b63-b4a1-43b8-a66a-6db8398d4254
title: 'rim.pawn: scripts read a pawn''s place, needs, skills, traits and job'
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: m
layer: engine
area: scripting
pillar:
- plugin-first
---

## Why

Scripts can count pawns but not read one. Mood, social and mourning all need to
ask where a person is, how hungry they are and what they are doing, and so
does any mod that reacts to people (DESIGN.md §4g).

## What

`rim.pawn(id)` returns a read-only view; `rim.pawns(filter)` lists ids in id
order. Reads only: writes go through commands, orders and thoughts.

## Acceptance criteria

- [ ] `rim.pawn(id)` exposes position, level, room, faction, needs, skills, traits and current job
- [ ] `rim.pawns({ faction =, room =, near = })` returns ids in id order
- [ ] Types and docs generated from the declaration
- [ ] A view costs no allocation beyond its table; bench shows the cost per call
