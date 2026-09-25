---
id: 38722b6e-b300-4282-93d7-b1399a6f60eb
title: 'Replays: run an epoch from its root, checked at every checkpoint'
type: feature
status: done
milestone: persistence
assignee: Oddur Sigurdsson
depends_on:
- d0524477-919e-4bd6-952e-95ff0d6bb58d
created: 2026-09-22
updated: 2026-09-24
closed_at: 2026-09-24
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

- [x] Replay reaches the same state hash at every checkpoint
- [x] A divergence names the tick and the sections that differ

## 2026-09-24

savefile::replay(path, mods, epoch) runs one epoch from its root: Sim::build from the seed (checked against the epoch's tick-0 snapshot first) or the epoch's first snapshot, then every log's commands at their ticks. Logs now keep a hash per snapshot section instead of one hash, so a disagreement names the sections (and fa6f0afb's per-mod desync hash is the same data). rim replay SAVE [--epoch N] [--mods DIR] in rim_client's cli.rs prints the checkpoints that agreed and exits 1 at the first that doesn't. Crash reports (8a8deb4a) can attach the save file as it is.

## 2026-09-24

Review: a save created after tick 0 recorded a seed root, so a replay from the seed reported a false divergence; such saves are now rooted at their snapshot. replay refuses an epoch from another engine version. The CLI rejects a bad --epoch, a missing --mods and a second save path.
