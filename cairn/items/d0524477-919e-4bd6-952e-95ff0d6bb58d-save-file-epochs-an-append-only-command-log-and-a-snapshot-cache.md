---
id: d0524477-919e-4bd6-952e-95ff0d6bb58d
title: 'Save file: epochs, an append-only command log and a snapshot cache'
type: feature
status: done
milestone: persistence
assignee: Oddur Sigurdsson
depends_on:
- b04a908e-8251-4d3a-a5a8-638c9b1759db
- c5d185be-9bb8-491d-b73d-94f75e4018f0
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
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

- [x] Commands are appended as they are applied; killing the process loses at most the unflushed tail
- [x] Load = newest snapshot + replay of the log after it, hash-checked at every checkpoint
- [x] Unchanged sections are shared between snapshots (test: second snapshot of an idle map is small)
- [x] A changed engine or mod list starts a new epoch rooted at the migrated snapshot
- [x] Test: a truncated file loads up to its last whole chunk

## 2026-09-24

From c5d185be: snapshots store DefIds raw, indexing engine:defs (each kind's qualified ids in DefId order). Snapshot::restore refuses a different def table. The epoch boundary must build a per-kind remap from those tables and apply it to every DefId field (Pawn.def/needs/carry, Job::Comfort.need, Thing.def, Blueprint.cost, MadeOf, Designated, map terrain, fields by index), or drop what no longer resolves (b4ad855e).

## 2026-09-24

Built as crates/rim_sim/src/savefile.rs. Chunks: kind, length, payload, checksum; reading stops at the first bad one and load cuts it off. Epoch chunks (lock, engine version, root), section chunks (zstd, keyed by hash, written once), snapshot chunks (header plus section hashes), log chunks (commands applied since the last log, each with its tick, and the full snapshot hash at the log's tick). Every epoch writes a snapshot at its start, so a load never regenerates the map; Root::Seed is kept for replays. Sim::record/take_applied give the log. A mod or engine change loads the newest snapshot (Snapshot::restore now accepts a different mod list if the def table is unchanged) and opens a new epoch; the log after that snapshot is reported as lost, so the client should snapshot on quit. A log whose hash disagrees on replay resumes at the last log that agreed and opens a new epoch. Def remapping across a def change is still refused: 0139/0063 take it. Cost: a full snapshot hash per log, 0.4-0.7 ms.

## 2026-09-24

Review: a write that failed partway left a torn chunk that later good chunks sat behind, and the next load's tail cut deleted them. Writes now go through one put() that cuts a failed write back off (or refuses further writes if it can't), commands are cleared only after their log is on disk, a section counts as stored only once written, and a load copies a damaged file to .damaged before cutting it. A too-short section chunk is an error, not a panic. Write failures themselves aren't tested: there's no portable way to make a write fail.
