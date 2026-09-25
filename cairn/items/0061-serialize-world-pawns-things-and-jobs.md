---
id: c5d185be-9bb8-491d-b73d-94f75e4018f0
title: 'Snapshot: the world as named sections, loaded back exactly'
type: feature
status: done
milestone: persistence
assignee: Oddur Sigurdsson
depends_on:
- b04a908e-8251-4d3a-a5a8-638c9b1759db
created: 2026-09-22
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

- [x] Save, load and save again gives the same bytes
- [x] Save at a tick, load (entities in reverse order), carry on: later snapshots equal the never-saved run's
- [x] The round trip runs on the crosscheck scenario in CI on every platform
- [x] Encoding measured and chosen; numbers recorded in a note

## 2026-09-24

Format per DESIGN.md §7a: named components, qualified def ids, save-local entity numbers, derived state rebuilt, CBOR vs MessagePack + zstd to be measured here (size and load time on a year-old colony). Add the round-trip test: save at tick N, load, run M ticks; state hashes must equal the unsaved run's, in the crosscheck scenario on every platform.

## 2026-09-24

Built as crates/rim_sim/src/snapshot.rs. Sections: engine:defs (each kind's qualified ids in DefId order: the string table), engine:world (rng, next entity id, wealth, reservations, messages), engine:map (terrain), engine:fields, engine:scripts (hooks switched off for running away, by registration index), one section per component keyed by entity id, and <mod>:data per mod. DefIds stay raw indices into engine:defs; a load refuses different mods or a different def table, since remapping belongs to the epoch boundary (d0524477). Rebuilt on load: map entity layers and door owners from the things, regions, rooms, field stamps, room boundaries, the pawn list. Two traps: room heat was a float sum in emitter insertion order (completion order, which a load can't reproduce), now an integer sum; and room values are indexed by room id, so the save keeps the id grid they were carried from, and a load mid-rebuild lets the fields carry them over on the next update exactly as the live game would. Measured (docs/engineering/dependencies.md): MessagePack beats CBOR (same size compressed, 2.6x faster decode); a year-old 250x250 colony is 38 KB, captures in 0.7 ms and restores in 5.2 ms, faster than a new game. The crosscheck example now runs a twin that is saved and loaded every 5 days and must match the unsaved game every day for a year; tests/snapshot.rs does 8 days on 4 seeds with the ECS reversed after the load, and caught a deliberately dropped door owner.

## 2026-09-24

Review found four ways a load diverged, each now with a failing-first test in tests/snapshot.rs: events a handler raises on the last tick were dropped (now saved in engine:world); commands applied before regions refreshed, so a live command could see stale regions a load can't reproduce (Sim::step now refreshes regions before applying commands; not rooms, since a second room rebuild between field updates would mis-pair carried room values); an unprefixed script data key came back with a ':' (round-trips exactly now); and items that declare emit never got emitters live but did after a load (place_item now registers them, which is what a mod item with emit expects).
