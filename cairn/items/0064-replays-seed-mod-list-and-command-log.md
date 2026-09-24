---
id: 38722b6e-b300-4282-93d7-b1399a6f60eb
title: 'Replays: run an epoch from its root, checked at every checkpoint'
type: feature
status: backlog
milestone: persistence
depends_on:
- d0524477-919e-4bd6-952e-95ff0d6bb58d
created: 2026-09-22
updated: 2026-09-22
priority: p1
api: none
effort: m
layer: engine
area: save
pillar:
- determinism
---

## Why

Bug reports become exact reproductions. In the save file a replay is just an
epoch: its root, its lockfile and its log (DESIGN.md §7a).

## What

A headless runner (`--replay save.rim`) that runs an epoch from its root and
checks the state hash at every checkpoint the log recorded, reporting the
first tick that diverges.

## Acceptance criteria

- [ ] Replay reaches the same state hash at every checkpoint
- [ ] A divergence names the tick and the sections that differ
