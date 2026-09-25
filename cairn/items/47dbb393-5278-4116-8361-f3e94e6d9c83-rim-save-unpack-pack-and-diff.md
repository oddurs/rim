---
id: 47dbb393-5278-4116-8361-f3e94e6d9c83
title: rim save unpack, pack and diff
type: feature
status: done
milestone: persistence
assignee: Oddur Sigurdsson
depends_on:
- d0524477-919e-4bd6-952e-95ff0d6bb58d
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
priority: p1
api: none
effort: m
layer: tooling
area: save
pillar:
- plugin-first
---

## Why

Saves should be readable by people: edit a pawn by hand, attach a readable
save to a bug report, diff two saves when co-op desyncs, and start a
`rim test` scene from a real colony (DESIGN.md §7a).

## What

- `unpack save.rim dir/`: one text file per section, qualified def ids, Uids.
- `pack dir/ save.rim`: lossless back; an edited snapshot becomes the root of a new epoch.
- `diff a.rim b.rim`: section by section, naming the first entity that differs.

## Acceptance criteria

- [x] unpack then pack gives a save that loads to the same state hash
- [x] An edited snapshot packs into a new epoch, and the old log is kept as history
- [x] diff names the section and the entity of the first difference

## 2026-09-24

Text is JSON through each section's Rust type, so an untouched section packs to the same bytes and pack tells an edit by Snapshot::hash. Defs are named via a path table in savetext.rs (DEF_REFS); a def field added to a component without an entry there shows as a number, which still packs. Script data has its own JSON form (Data branches on is_human_readable; MessagePack unchanged). Entities show as hecs bits (generation 1 << 32 | id), the same number everywhere they appear. A NaN in script data has no JSON form and fails the unpack.
