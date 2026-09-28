---
id: 66617791-c3fe-4fbf-b2ae-073ffab471c1
title: A death leaves a body that remembers who it was
type: feature
status: backlog
milestone: story
created: 2026-09-26
updated: 2026-09-27
priority: p0
api: additive
effort: m
layer: engine
area: sim
pillar:
- survival
---

## Why

A pawn is despawned at death, so there is nothing to bury, grieve over or
step past. `pawn_died` has no killer, though the pawn knows its last attacker
(DESIGN.md §4g).

## Acceptance criteria

- [ ] Death spawns a corpse thing carrying the person's id, name and creature
- [ ] Corpses can be hauled and rot over days; core defines the corpse def
- [ ] `pawn_died` carries the killer and the corpse id
- [ ] Saves keep corpses; a replay test covers a death, a haul and rot

## 2026-09-27

People (5d09b04e, DESIGN.md §6h): a corpse draws with the lying pose (6cb4f2f9) of the person it was, its colours faded.
