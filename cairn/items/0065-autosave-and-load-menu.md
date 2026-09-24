---
id: 01e86e7b-17ac-4e5a-8920-d72a438037f1
title: 'Always saved: autosnapshots off the sim thread, and a load menu'
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
layer: client
area: ui
---

## Why

Never lose a colony. The log is already on disk as you play (DESIGN.md §7a);
what's left is snapshots often enough that a load is quick, and a way to pick
a colony.

## What

- A snapshot every few in-game hours: copy the sections' data on the sim
  thread, encode and write on a worker, so the game doesn't hitch.
- Compaction keeps the last few snapshots.
- Main menu: continue, and load any colony with its name, day and colonists.

## Acceptance criteria

- [ ] Snapshots are taken on a cadence without a frame over budget
- [ ] Old snapshots are compacted away
- [ ] Load from main menu
