---
id: c5d185be-9bb8-491d-b73d-94f75e4018f0
title: 'Snapshot: the world as named sections, loaded back exactly'
type: feature
status: backlog
milestone: persistence
depends_on:
- b04a908e-8251-4d3a-a5a8-638c9b1759db
created: 2026-09-22
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

A snapshot is the cache that makes loading fast (DESIGN.md §7a): the whole
world as sections by owner and name, loaded back so exactly that carrying on
can't tell it was ever saved.

## What

- Sections: `engine:rng`, `engine:map`, the field stock grids and pushes,
  the calendar, script data per mod, messages, colony flags.
- Components one section each (`engine:pawn`, `engine:thing`,
  `engine:blueprint`, …), keyed by Uid, with jobs and reservations
  translated from hecs handles to Uids.
- Qualified def ids through a string table; nothing derived is saved, and
  regions, rooms, paths and the wealth cache are rebuilt on load.
- Each section's hash over its canonical bytes.
- CBOR vs MessagePack, each with zstd, measured on a year-old colony
  (size, encode and load time) and the winner recorded here.

## Acceptance criteria

- [ ] Save, load and save again gives the same bytes
- [ ] Save at a tick, load (entities in reverse order), carry on: later snapshots equal the never-saved run's
- [ ] The round trip runs on the crosscheck scenario in CI on every platform
- [ ] Encoding measured and chosen; numbers recorded in a note

## 2026-09-24

Format per DESIGN.md §7a: named components, qualified def ids, save-local entity numbers, derived state rebuilt, CBOR vs MessagePack + zstd to be measured here (size and load time on a year-old colony). Add the round-trip test: save at tick N, load, run M ticks; state hashes must equal the unsaved run's, in the crosscheck scenario on every platform.
