---
id: 354
uid: 6412b135-68c0-4155-8716-ce2182d71435
title: 'Witnesses: who saw an event'
type: feature
status: backlog
milestone: story
depends_on:
- 400
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: m
layer: engine
area: sim
pillar:
- determinism
---

## Why

Mood (only those who saw the body feel dread), social (shared experience),
dialogue (people only talk about what they know) and the chronicle all need to
agree on who perceived an event (DESIGN.md §4g).

## What

An event def opts in with a radius. Witnesses are pawns within it, on its
level, in its room or with a clear line to it. Computed once at emit time, in
id order, into the payload. Levels come from the depth work (Positions gain z)
once it lands; until then everything is level 0.

## Acceptance criteria

- [ ] Event defs declare `witnessed = { radius = }`; others pay nothing
- [ ] Same room, or a clear line on the same level
- [ ] Witness ids in id order in the payload and in the memory log
- [ ] `pawn_died` asks for witnesses
- [ ] Bench: cost per witnessed event on the target map
