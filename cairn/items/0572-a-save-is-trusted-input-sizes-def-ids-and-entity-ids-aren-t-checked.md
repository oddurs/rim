---
id: 572
uid: 16b78528-d634-4fef-a73f-2c99173a5dbf
title: 'A save is trusted input: sizes, def ids and entity ids aren''t checked'
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: engine
area: save
---

## What

Found in the save/load review:
- `ws.width`, `height`, `below` and `above` go straight to allocation (snapshot.rs ~419).
- Def ids aren't bounds-checked when the def tables match exactly. `Remap::get` returns `Some(d)` as-is, and then `defs.creature(p.def)` (~528) panics.
- A `next_entity` at or below the highest saved id lets `World::spawn` overwrite a live entity.
- `zstd::decode_all` has no size cap.
- After a mod is removed, `Pawn.worn` isn't checked the way `hand` and `Held` are (~785-802).

Chunk checksums stop random damage, but not `savetext pack` edits.

## How it fails

A hand-edited or damaged save panics or corrupts the game instead of failing to load with a message.

## Fix

Bound sizes; check every def id against its table; set `next_entity` to at least the highest id plus one; cap decompression; fix up `worn` as `hand` is.

## Acceptance

- [ ] Each case fails the load with a message, or is repaired
- [ ] A test per case
