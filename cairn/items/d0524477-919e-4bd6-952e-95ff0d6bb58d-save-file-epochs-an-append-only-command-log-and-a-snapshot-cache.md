---
id: d0524477-919e-4bd6-952e-95ff0d6bb58d
title: 'Save file: epochs, an append-only command log and a snapshot cache'
type: feature
status: backlog
milestone: persistence
depends_on:
- b04a908e-8251-4d3a-a5a8-638c9b1759db
- c5d185be-9bb8-491d-b73d-94f75e4018f0
created: 2026-09-24
updated: 2026-09-24
priority: p0
api: none
effort: l
layer: engine
area: save
pillar:
- determinism
---

## Why

DESIGN.md §7a: the log is the save, snapshots are a cache, and an epoch
starts when the code changes. This is the file that holds all three.

## What

- An epoch: the mod lockfile, the engine version, a root (the seed, or a
  snapshot) and the commands since, each at its tick.
- The game appends commands as they are applied; there is no "save" step.
- Snapshot sections stored by hash, so an unchanged section is written once.
- Append-only chunks with checksums: a crash cuts the tail, never the file.
- Load: the newest snapshot of the last epoch, then replay the log after it,
  checking the state hash at every checkpoint. On a mismatch the snapshot
  wins and the mismatch is reported.
- A new epoch when the engine or mod list differs from the save's.

## Acceptance criteria

- [ ] Commands are appended as they are applied; killing the process loses at most the unflushed tail
- [ ] Load = newest snapshot + replay of the log after it, hash-checked at every checkpoint
- [ ] Unchanged sections are shared between snapshots (test: second snapshot of an idle map is small)
- [ ] A changed engine or mod list starts a new epoch rooted at the migrated snapshot
- [ ] Test: a truncated file loads up to its last whole chunk

## 2026-09-24

From c5d185be: snapshots store DefIds raw, indexing engine:defs (each kind's qualified ids in DefId order). Snapshot::restore refuses a different def table. The epoch boundary must build a per-kind remap from those tables and apply it to every DefId field (Pawn.def/needs/carry, Job::Comfort.need, Thing.def, Blueprint.cost, MadeOf, Designated, map terrain, fields by index), or drop what no longer resolves (b4ad855e).
