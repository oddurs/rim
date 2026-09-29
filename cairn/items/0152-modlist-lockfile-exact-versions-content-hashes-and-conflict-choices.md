---
id: 152
uid: 9e979a26-5dc0-4122-b62d-fc48f14d8488
title: 'Modlist lockfile: exact versions, content hashes and conflict choices'
type: feature
status: backlog
milestone: platform
depends_on:
- 151
created: 2026-09-23
updated: 2026-09-27
priority: p0
api: none
effort: m
layer: engine
area: modding
pillar:
- determinism
- plugin-first
---

## Why

One file that says exactly what was loaded: the base of replays, bug reports, crash reports and the co-op handshake. A modpack is a lockfile someone shared (DESIGN.md §10).

## Acceptance criteria

- [ ] `mods.lock` records each mod's id, version, source and content hash
- [ ] Records the player's picks for contested slots (dbb92ebe), each keyed by the slot and its contenders' versions
- [ ] Separates the colony (sim-side mods, their sim hashes, colony option values, sim picks) from the player's client-side mods (e4b96647); only the colony part is compared by saves, seed codes and co-op
- [ ] Loading with a lockfile reproduces the same def database byte-for-byte
- [ ] Replays and crash reports embed it

## 2026-09-27

Three new items write into what becomes the lockfile. A set (73751f4f) is a lockfile that ships with rim: this item gives it versions and hashes. Sim options (fe54d733) are recorded in the epoch until the lockfile lands, then move here. The storyteller pick (a77aec3a) is a claim pick, which the claims item already says the lockfile takes over.

## 2026-09-27

Decided 2026-09-27 (modding review, PR #246): the lockfile hashes every file of every mod, for integrity. The save's epoch keys only on each mod's sim side (173de74c). Those are two jobs. A lockfile's colony section covers sim-side mods, colony options and sim picks; client-side mods (e4b96647) are listed apart and never compared. The earlier note calling the storyteller pick a claim pick is superseded: the storyteller is a singleton kind (a77aec3a), and two replacements are an ordinary contested field.
