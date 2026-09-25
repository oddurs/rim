---
id: 01e86e7b-17ac-4e5a-8920-d72a438037f1
title: 'Always saved: the client logs and snapshots off the sim thread'
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
layer: client
area: ui
---

## Why

Never lose a colony. The log is already on disk as you play (DESIGN.md §7a);
what's left is snapshots often enough that a load is quick, and a way to pick
a colony.

## What

- Every new game gets a save file; the client logs every 600 ticks and
  takes a snapshot every six in-game hours and on quit.
- The sim thread only captures (under a millisecond); a writer thread
  compresses and writes.
- Compaction keeps each epoch's root snapshot, the newest few, and every
  log, so replays from the seed still work.
- `rim --continue` and `rim --load FILE` bring a colony back. The menu in
  front of them is af316e9d.

## Acceptance criteria

- [x] Snapshots are taken on a cadence without a frame over budget
- [x] Old snapshots are compacted away
- [x] Continue or load a colony (the menu itself is af316e9d)

## 2026-09-24

Review: compact() would rewrite a file with a damaged middle from its readable prefix, losing the good chunks after the damage with no copy; it now refuses a damaged file (the load cuts it and keeps a .damaged copy), and the client only compacts undamaged saves. The Writer cleared commands before its write succeeded; a failed log's commands now ride in the next log, and a snapshot is written even if its log failed. Save failures reach view.warnings ('this colony isn't being saved') and finish() returns the error. prevent_quit only once the UI is up; a bare --load is an error; --seed with a load is warned about. Verified end to end in the real client (HOME pointed at a scratch dir): new game killed after 20 s, --continue replayed 600 ticks from the seed, killed again, rim replay agreed through tick 1800.
